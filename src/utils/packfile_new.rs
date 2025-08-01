use std::collections::HashMap;
use std::path::PathBuf;
use crate::{GitError, Result};
use byteorder::{BigEndian, ReadBytesExt};
use std::io::{self, Cursor, Read, Write};

/// 精确跟踪 zlib 流消耗字节数的解码器
struct PreciseZlibDecoder<'a> {
    data: &'a [u8],
    total_in: usize,
}

impl<'a> PreciseZlibDecoder<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { 
            data, 
            total_in: 0,
        }
    }
    
    fn decompress(&mut self, expected_size: usize) -> Result<Vec<u8>> {
        use flate2::{Decompress, FlushDecompress};
        
        let mut decompressor = Decompress::new(true); // true for zlib format
        let mut output = vec![0u8; expected_size];
        let mut input_consumed = 0;
        let mut output_produced = 0;
        
        // 逐步解压缩，直到得到期望的输出大小
        while output_produced < expected_size && input_consumed < self.data.len() {
            let chunk_size = std::cmp::min(1024, self.data.len() - input_consumed);
            let input_chunk = &self.data[input_consumed..input_consumed + chunk_size];
            
            match decompressor.decompress(
                input_chunk,
                &mut output[output_produced..],
                FlushDecompress::None
            ) {
                Ok(status) => {
                    input_consumed += status.bytes_consumed;
                    output_produced += status.bytes_written;
                    
                    // 如果解压缩完成，记录确切的输入字节数
                    if status.bytes_written == 0 && status.bytes_consumed == 0 {
                        break;
                    }
                }
                Err(e) => {
                    return Err(GitError::invalid_command(format!("Zlib decompression error: {}", e)));
                }
            }
        }
        
        if output_produced != expected_size {
            return Err(GitError::invalid_command(format!(
                "Decompression size mismatch: expected {}, got {}", 
                expected_size, 
                output_produced
            )));
        }
        
        self.total_in = input_consumed;
        output.truncate(output_produced);
        Ok(output)
    }
    
    fn bytes_consumed(&self) -> usize {
        self.total_in
    }
}

/// Packfile 处理器
pub struct PackfileProcessor {
    gitdir: PathBuf,
}

#[derive(Debug)]
struct ObjectData {
    obj_type: u8,
    data: Vec<u8>,
}

#[derive(Debug)]
struct PackfileObject {
    hash: String,
    obj_type: u8,
    data: Vec<u8>,
}

impl PackfileProcessor {
    pub fn new(gitdir: PathBuf) -> Self {
        PackfileProcessor { gitdir }
    }
    
    /// 处理 packfile 数据并将对象写入仓库
    pub fn process_packfile(&self, packfile_data: &[u8]) -> Result<Vec<String>> {
        println!("DEBUG: process_packfile called with {} bytes", packfile_data.len());
        
        if packfile_data.len() < 12 {
            return Err(GitError::invalid_command("Invalid packfile: too short".to_string()));
        }
        
        let mut cursor = Cursor::new(packfile_data);
        
        // 验证packfile头部签名
        let mut signature = [0u8; 4];
        cursor.read_exact(&mut signature)?;
        if &signature != b"PACK" {
            return Err(GitError::invalid_command("Invalid packfile signature".to_string()));
        }
        println!("DEBUG: Valid PACK header found");
        
        // 读取版本号
        let version = cursor.read_u32::<BigEndian>()?;
        println!("DEBUG: Packfile version: {}", version);
        if version != 2 {
            return Err(GitError::invalid_command(format!("Unsupported packfile version: {}", version)));
        }
        
        // 读取对象数量
        let object_count = cursor.read_u32::<BigEndian>()?;
        println!("DEBUG: Object count: {}", object_count);
        
        let mut objects = Vec::new();
        let mut created_hashes = Vec::new();
        
        // 解析每个对象
        for i in 0..object_count {
            println!("DEBUG: Processing object {} of {}", i + 1, object_count);
            let obj = self.read_object(&mut cursor, i)?;
            
            // 计算对象hash
            let hash = self.calculate_object_hash(&obj)?;
            
            // 写入对象到仓库
            self.write_object(&hash, &obj)?;
            
            objects.push(PackfileObject {
                hash: hash.clone(),
                obj_type: obj.obj_type,
                data: obj.data,
            });
            
            created_hashes.push(hash);
        }
        
        println!("DEBUG: Successfully processed {} objects", created_hashes.len());
        Ok(created_hashes)
    }
    
    fn read_object(&self, cursor: &mut Cursor<&[u8]>, index: u32) -> Result<ObjectData> {
        // 读取对象头部
        let (obj_type, size) = self.read_object_header(cursor)?;
        println!("DEBUG: Object {}: type={}, size={}", index, obj_type, size);
        
        match obj_type {
            0 => {
                // 无效的对象类型，检查数据
                let pos = cursor.position();
                println!("DEBUG: Invalid object type 0 at position {}", pos);
                return Err(GitError::invalid_command(format!("Invalid object type: {} at position {}", obj_type, pos)));
            }
            1..=4 => {
                // 直接对象类型 (commit, tree, blob, tag)
                let compressed_data = self.read_compressed_data(cursor, size)?;
                Ok(ObjectData {
                    obj_type,
                    data: compressed_data,
                })
            }
            6 => {
                // OFS_DELTA - offset delta
                println!("DEBUG: Reading OFS_DELTA offset at position {}", cursor.position());
                let offset = self.read_offset_encoding(cursor)?;
                println!("DEBUG: OFS_DELTA offset: {}, now at position {}", offset, cursor.position());
                let compressed_data = self.read_compressed_data(cursor, size)?;
                // TODO: 实现delta解码
                Ok(ObjectData {
                    obj_type,
                    data: compressed_data,
                })
            }
            7 => {
                // REF_DELTA - reference delta
                let mut base_hash = [0u8; 20];
                cursor.read_exact(&mut base_hash)?;
                let compressed_data = self.read_compressed_data(cursor, size)?;
                // TODO: 实现delta解码
                Ok(ObjectData {
                    obj_type,
                    data: compressed_data,
                })
            }
            _ => Err(GitError::invalid_command(format!("Unknown object type: {}", obj_type))),
        }
    }
    
    fn read_object_header(&self, cursor: &mut Cursor<&[u8]>) -> Result<(u8, usize)> {
        let pos_before = cursor.position();
        let mut byte = cursor.read_u8()?;
        let obj_type = (byte >> 4) & 7;
        let mut size = (byte & 15) as usize;
        let mut shift = 4;
        
        println!("DEBUG: read_object_header at pos {}: first_byte=0b{:08b} ({}), obj_type={}, initial_size={}", 
                 pos_before, byte, byte, obj_type, size);
        
        while byte & 0x80 != 0 {
            byte = cursor.read_u8()?;
            size |= ((byte & 0x7f) as usize) << shift;
            shift += 7;
            println!("DEBUG: Additional size byte: 0b{:08b} ({}), new_size={}", byte, byte, size);
        }
        
        println!("DEBUG: Final object header: type={}, size={}", obj_type, size);
        Ok((obj_type, size))
    }
    
    fn read_offset_encoding(&self, cursor: &mut Cursor<&[u8]>) -> Result<u64> {
        let mut byte = cursor.read_u8()?;
        let mut offset = (byte & 0x7f) as u64;
        
        while byte & 0x80 != 0 {
            byte = cursor.read_u8()?;
            offset = ((offset + 1) << 7) | (byte & 0x7f) as u64;
        }
        
        Ok(offset)
    }
    
    fn read_compressed_data(&self, cursor: &mut Cursor<&[u8]>, expected_size: usize) -> Result<Vec<u8>> {
        let start_pos = cursor.position() as usize;
        println!("DEBUG: read_compressed_data at pos {}, expected_size={}", start_pos, expected_size);
        
        let remaining_data = &cursor.get_ref()[start_pos..];
        
        // 使用精确的 zlib 解码器
        let mut decoder = PreciseZlibDecoder::new(remaining_data);
        let decompressed = decoder.decompress(expected_size)?;
        let bytes_consumed = decoder.bytes_consumed();
        
        println!("DEBUG: Successfully decompressed {} bytes using {} compressed bytes (precise)", 
                 decompressed.len(), bytes_consumed);
        
        // 更新cursor位置
        let new_pos = start_pos + bytes_consumed;
        cursor.set_position(new_pos as u64);
        println!("DEBUG: Updated cursor position to {}", new_pos);
        
        Ok(decompressed)
    }
    
    fn calculate_object_hash(&self, obj: &ObjectData) -> Result<String> {
        use sha1::{Sha1, Digest};
        
        let type_name = match obj.obj_type {
            1 => "commit",
            2 => "tree", 
            3 => "blob",
            4 => "tag",
            _ => return Err(GitError::invalid_command(format!("Invalid object type: {}", obj.obj_type))),
        };
        
        let header = format!("{} {}\0", type_name, obj.data.len());
        
        let mut hasher = Sha1::new();
        hasher.update(header.as_bytes());
        hasher.update(&obj.data);
        
        Ok(hex::encode(hasher.finalize()))
    }
    
    fn write_object(&self, hash: &str, obj: &ObjectData) -> Result<()> {
        let obj_path = crate::utils::fs::obj_to_pathbuf(&self.gitdir, hash);
        
        // 如果对象已存在，跳过
        if obj_path.exists() {
            return Ok(());
        }
        
        // 创建目录
        if let Some(parent) = obj_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // 构建完整的对象内容
        let type_name = match obj.obj_type {
            1 => "commit",
            2 => "tree",
            3 => "blob", 
            4 => "tag",
            _ => return Err(GitError::invalid_command(format!("Invalid object type: {}", obj.obj_type))),
        };
        
        let header = format!("{} {}\0", type_name, obj.data.len());
        let mut full_content = header.into_bytes();
        full_content.extend_from_slice(&obj.data);
        
        // 压缩并写入
        let compressed = crate::utils::fs::compress_object(&full_content)?;
        std::fs::write(&obj_path, compressed)?;
        
        Ok(())
    }
}
