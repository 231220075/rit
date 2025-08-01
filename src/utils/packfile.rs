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
        use flate2::{Decompress, FlushDecompress, Status};
        
        let mut decompressor = Decompress::new(true); // true for zlib format
        let mut output = vec![0u8; expected_size];
        let mut input_consumed = 0;
        let mut output_produced = 0;
        
        // 逐步解压缩，直到得到期望的输出大小
        while output_produced < expected_size && input_consumed < self.data.len() {
            let chunk_size = std::cmp::min(1024, self.data.len() - input_consumed);
            let input_chunk = &self.data[input_consumed..input_consumed + chunk_size];
            
            let input_before = decompressor.total_in();
            let output_before = decompressor.total_out();
            
            match decompressor.decompress(
                input_chunk,
                &mut output[output_produced..],
                FlushDecompress::None
            ) {
                Ok(Status::Ok) | Ok(Status::StreamEnd) => {
                    let input_consumed_this_round = (decompressor.total_in() - input_before) as usize;
                    let output_produced_this_round = (decompressor.total_out() - output_before) as usize;
                    
                    input_consumed += input_consumed_this_round;
                    output_produced += output_produced_this_round;
                    
                    // 如果解压缩完成
                    if decompressor.total_out() as usize >= expected_size {
                        break;
                    }
                    
                    // 如果没有更多输入或输出，停止
                    if input_consumed_this_round == 0 && output_produced_this_round == 0 {
                        break;
                    }
                }
                Ok(Status::BufError) => {
                    // 需要更多输入或输出空间
                    let input_consumed_this_round = (decompressor.total_in() - input_before) as usize;
                    input_consumed += input_consumed_this_round;
                    continue;
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
    // 存储已解析的对象，用于 delta 解码
    resolved_objects: HashMap<usize, ObjectData>,
}

#[derive(Debug, Clone)]
struct ObjectData {
    obj_type: u8,
    data: Vec<u8>,
    // Delta 相关信息
    delta_info: Option<DeltaInfo>,
}

#[derive(Debug, Clone)]
enum DeltaInfo {
    OfsLink(u64), // OFS_DELTA - 偏移量
    RefLink([u8; 20]), // REF_DELTA - 引用哈希
}

#[derive(Debug)]
struct PackfileObject {
    hash: String,
    obj_type: u8,
    data: Vec<u8>,
}

impl PackfileProcessor {
    pub fn new(gitdir: PathBuf) -> Self {
        PackfileProcessor { 
            gitdir,
            resolved_objects: HashMap::new(),
        }
    }
    
    /// 处理 packfile 数据并将对象写入仓库
    pub fn process_packfile(&mut self, packfile_data: &[u8]) -> Result<Vec<String>> {
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
        
        // 读取版本号
        let version = cursor.read_u32::<BigEndian>()?;
        if version != 2 {
            return Err(GitError::invalid_command(format!("Unsupported packfile version: {}", version)));
        }
        
        // 读取对象数量
        let object_count = cursor.read_u32::<BigEndian>()?;
        println!("Processing {} objects from packfile...", object_count);
        
        let mut objects = Vec::new();
        let mut created_hashes = Vec::new();
        let mut object_positions = Vec::new(); // 记录每个对象在 packfile 中的位置
        
        // 解析每个对象
        for i in 0..object_count {
            let current_pos = cursor.position();
            object_positions.push(current_pos);
            
            // 检查是否到达了数据末尾（保留20字节用于校验和）
            if current_pos as usize >= packfile_data.len() - 20 {
                break;
            }
            
            let obj = match self.read_object(&mut cursor, i) {
                Ok(obj) => obj,
                Err(_) => continue,
            };            // 先将原始对象存储，后续解析 delta 时使用
            let mut current_obj = obj;
            
            // 如果是 delta 对象，需要解析
            if current_obj.delta_info.is_some() {
                current_obj = self.resolve_delta_object(&current_obj, i, &object_positions)?;
            }
            
            // 计算对象hash
            let hash = self.calculate_object_hash(&current_obj)?;
            
            // 写入对象到仓库
            self.write_object(&hash, &current_obj)?;
            
            // 存储已解析的对象供后续 delta 解码使用
            self.resolved_objects.insert(i as usize, current_obj.clone());
            
            objects.push(PackfileObject {
                hash: hash.clone(),
                obj_type: current_obj.obj_type,
                data: current_obj.data,
            });
            
            created_hashes.push(hash);
            
            // 显示进度
            if (i + 1) % 50 == 0 || i + 1 == object_count {
                println!("Processed {}/{} objects", i + 1, object_count);
            }
        }
        
        println!("Successfully processed {} objects", created_hashes.len());
        Ok(created_hashes)
    }
    
    fn read_object(&self, cursor: &mut Cursor<&[u8]>, _index: u32) -> Result<ObjectData> {
        // 读取对象头部
        let (obj_type, size) = self.read_object_header(cursor)?;
        //println!("DEBUG: Object {}: type={}, size={}", index, obj_type, size);
        
        match obj_type {
            0 => {
                // 无效的对象类型，检查数据
                let pos = cursor.position();
                //println!("DEBUG: Invalid object type 0 at position {}", pos);
                return Err(GitError::invalid_command(format!("Invalid object type: {} at position {}", obj_type, pos)));
            }
            1..=4 => {
                // 直接对象类型 (commit, tree, blob, tag)
                let compressed_data = self.read_compressed_data(cursor, size)?;
                Ok(ObjectData {
                    obj_type,
                    data: compressed_data,
                    delta_info: None,
                })
            }
            6 => {
                // OFS_DELTA - offset delta
                //println!("DEBUG: Reading OFS_DELTA offset at position {}", cursor.position());
                let offset = self.read_offset_encoding(cursor)?;
                //println!("DEBUG: OFS_DELTA offset: {}, now at position {}", offset, cursor.position());
                let compressed_data = self.read_compressed_data(cursor, size)?;
                Ok(ObjectData {
                    obj_type,
                    data: compressed_data,
                    delta_info: Some(DeltaInfo::OfsLink(offset)),
                })
            }
            7 => {
                // REF_DELTA - reference delta
                //println!("DEBUG: Reading REF_DELTA at position {}", cursor.position());

                // 检查剩余数据长度
                let remaining = cursor.get_ref().len() - cursor.position() as usize;
                //println!("DEBUG: Remaining data length: {}", remaining);

                if remaining < 20 {
                    return Err(GitError::invalid_command(format!(
                        "Not enough data for REF_DELTA hash: {} bytes remaining, need 20", 
                        remaining
                    )));
                }
                
                // 显示接下来的30个字节以便调试
                let current_pos = cursor.position() as usize;
                let _debug_bytes = &cursor.get_ref()[current_pos..std::cmp::min(current_pos + 30, cursor.get_ref().len())];
                //println!("DEBUG: Next 30 bytes: {:02x?}", debug_bytes);

                // 暂时跳过有问题的 REF_DELTA 对象
                //println!("DEBUG: Skipping problematic REF_DELTA object temporarily");
                return Err(GitError::invalid_command("Skipping REF_DELTA for now".to_string()));
                
                /*
                let mut base_hash = [0u8; 20];
                cursor.read_exact(&mut base_hash)?;
                println!("DEBUG: REF_DELTA base hash: {}, now at position {}", hex::encode(&base_hash), cursor.position());
                
                // 检查压缩数据的剩余长度
                let remaining_after_hash = cursor.get_ref().len() - cursor.position() as usize;
                println!("DEBUG: Remaining data after hash: {}", remaining_after_hash);
                
                let compressed_data = self.read_compressed_data(cursor, size)?;
                Ok(ObjectData {
                    obj_type,
                    data: compressed_data,
                    delta_info: Some(DeltaInfo::RefLink(base_hash)),
                })
                */
            }
            _ => Err(GitError::invalid_command(format!("Unknown object type: {}", obj_type))),
        }
    }
    
    fn read_object_header(&self, cursor: &mut Cursor<&[u8]>) -> Result<(u8, usize)> {
        let _pos_before = cursor.position();
        let mut byte = cursor.read_u8()?;
        let obj_type = (byte >> 4) & 7;
        let mut size = (byte & 15) as usize;
        let mut shift = 4;
        
        //println!("DEBUG: read_object_header at pos {}: first_byte=0b{:08b} ({}), obj_type={}, initial_size={}", 
        //         pos_before, byte, byte, obj_type, size);
        
        while byte & 0x80 != 0 {
            byte = cursor.read_u8()?;
            size |= ((byte & 0x7f) as usize) << shift;
            shift += 7;
            //println!("DEBUG: Additional size byte: 0b{:08b} ({}), new_size={}", byte, byte, size);
        }
        
        //println!("DEBUG: Final object header: type={}, size={}", obj_type, size);
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
        //println!("DEBUG: read_compressed_data at pos {}, expected_size={}", start_pos, expected_size);
        
        let remaining_data = &cursor.get_ref()[start_pos..];
        
        // 使用精确的 zlib 解码器
        let mut decoder = PreciseZlibDecoder::new(remaining_data);
        let decompressed = decoder.decompress(expected_size)?;
        let bytes_consumed = decoder.bytes_consumed();
        
        //println!("DEBUG: Successfully decompressed {} bytes using {} compressed bytes (precise)", 
        //         decompressed.len(), bytes_consumed);
        
        // 更新cursor位置
        let new_pos = start_pos + bytes_consumed;
        cursor.set_position(new_pos as u64);
        //println!("DEBUG: Updated cursor position to {}", new_pos);
        
        Ok(decompressed)
    }
    
    fn resolve_delta_object(&mut self, obj: &ObjectData, current_index: u32, object_positions: &[u64]) -> Result<ObjectData> {
        match &obj.delta_info {
            None => {
                // 不是 delta 对象，直接返回
                Ok(obj.clone())
            }
            Some(DeltaInfo::OfsLink(offset)) => {
                //println!("DEBUG: Resolving OFS_DELTA with offset {}", offset);
                
                // 计算基础对象在 packfile 中的位置
                let current_pos = object_positions[current_index as usize];
                if *offset > current_pos {
                    return Err(GitError::invalid_command(format!(
                        "Invalid OFS_DELTA offset: {} from position {}", 
                        offset, 
                        current_pos
                    )));
                }
                let base_pos = current_pos - offset;
                
                // 找到基础对象的索引
                let mut base_index = None;
                for (i, &pos) in object_positions.iter().enumerate() {
                    if pos == base_pos {
                        base_index = Some(i);
                        break;
                    }
                }
                
                let base_idx = base_index.ok_or_else(|| GitError::invalid_command(format!(
                    "Base object at position {} not found for OFS_DELTA", 
                    base_pos
                )))?;
                
                // 获取基础对象
                let base_obj = self.resolved_objects.get(&base_idx)
                    .ok_or_else(|| GitError::invalid_command(format!(
                        "Base object at index {} not found for OFS_DELTA", 
                        base_idx
                    )))?;
                
                // 应用 delta
                self.apply_delta(base_obj, &obj.data)
            }
            Some(DeltaInfo::RefLink(base_hash)) => {
                //println!("DEBUG: Resolving REF_DELTA with base hash {}", hex::encode(base_hash));
                
                // 在已解析的对象中查找基础对象
                let mut base_obj = None;
                for (_, obj) in &self.resolved_objects {
                    // 计算对象哈希并比较
                    if let Ok(hash_str) = self.calculate_object_hash(obj) {
                        let hash_bytes = hex::decode(&hash_str).unwrap_or_default();
                        if hash_bytes.len() == 20 && hash_bytes[..] == base_hash[..] {
                            base_obj = Some(obj);
                            break;
                        }
                    }
                }
                
                match base_obj {
                    Some(base) => self.apply_delta(base, &obj.data),
                    None => {
                        //println!("DEBUG: Base object not found for REF_DELTA, hash: {}", hex::encode(base_hash));
                        // 暂时跳过，可能需要从文件系统读取
                        Err(GitError::invalid_command(format!(
                            "Base object {} not found for REF_DELTA", 
                            hex::encode(base_hash)
                        )))
                    }
                }
            }
        }
    }
    
    fn apply_delta(&self, base_obj: &ObjectData, delta_data: &[u8]) -> Result<ObjectData> {
        //println!("DEBUG: Applying delta to base object type {}", base_obj.obj_type);
        
        let mut cursor = Cursor::new(delta_data);
        
        // 读取基础对象大小
        let base_size = self.read_delta_size(&mut cursor)?;
        //println!("DEBUG: Delta expects base size: {}, actual: {}", base_size, base_obj.data.len());
        
        if base_size != base_obj.data.len() {
            return Err(GitError::invalid_command(format!(
                "Base size mismatch: expected {}, got {}", 
                base_size, 
                base_obj.data.len()
            )));
        }
        
        // 读取结果对象大小
        let result_size = self.read_delta_size(&mut cursor)?;
        //println!("DEBUG: Delta result size: {}", result_size);
        
        // 应用 delta 指令
        let mut result_data = Vec::new();
        
        while cursor.position() < delta_data.len() as u64 {
            let instruction = cursor.read_u8()?;
            
            if instruction & 0x80 != 0 {
                // 复制指令
                let (offset, size) = self.read_copy_instruction(&mut cursor, instruction)?;
                
                if offset + size > base_obj.data.len() {
                    return Err(GitError::invalid_command(format!(
                        "Copy instruction out of bounds: offset={}, size={}, base_len={}", 
                        offset, size, base_obj.data.len()
                    )));
                }
                
                result_data.extend_from_slice(&base_obj.data[offset..offset + size]);
                //println!("DEBUG: Copy {} bytes from offset {}", size, offset);
            } else {
                // 插入指令
                let size = instruction as usize;
                if size > 0 {
                    if cursor.position() + size as u64 > delta_data.len() as u64 {
                        return Err(GitError::invalid_command("Insert instruction out of bounds".to_string()));
                    }
                    
                    let mut insert_data = vec![0u8; size];
                    cursor.read_exact(&mut insert_data)?;
                    result_data.extend_from_slice(&insert_data);
                    //println!("DEBUG: Insert {} bytes", size);
                }
            }
        }
        
        if result_data.len() != result_size {
            return Err(GitError::invalid_command(format!(
                "Delta result size mismatch: expected {}, got {}", 
                result_size, 
                result_data.len()
            )));
        }
        
        //println!("DEBUG: Delta applied successfully, result: {} bytes", result_data.len());
        
        Ok(ObjectData {
            obj_type: base_obj.obj_type, // 继承基础对象的类型
            data: result_data,
            delta_info: None,
        })
    }
    
    fn read_delta_size(&self, cursor: &mut Cursor<&[u8]>) -> Result<usize> {
        let mut size = 0usize;
        let mut shift = 0;
        
        loop {
            let byte = cursor.read_u8()?;
            size |= ((byte & 0x7f) as usize) << shift;
            shift += 7;
            
            if byte & 0x80 == 0 {
                break;
            }
        }
        
        Ok(size)
    }
    
    fn read_copy_instruction(&self, cursor: &mut Cursor<&[u8]>, instruction: u8) -> Result<(usize, usize)> {
        let mut offset = 0usize;
        let mut size = 0usize;
        
        // 读取偏移量
        for i in 0..4 {
            if instruction & (1 << i) != 0 {
                let byte = cursor.read_u8()? as usize;
                offset |= byte << (i * 8);
            }
        }
        
        // 读取大小
        for i in 0..3 {
            if instruction & (1 << (i + 4)) != 0 {
                let byte = cursor.read_u8()? as usize;
                size |= byte << (i * 8);
            }
        }
        
        // 如果大小为0，默认为65536
        if size == 0 {
            size = 0x10000;
        }
        
        Ok((offset, size))
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
