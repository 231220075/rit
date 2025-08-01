use std::collections::HashMap;
use crate::{GitError, Result};
use reqwest::blocking::Client;
use std::time::Duration;

/// Git 网络协议支持
pub struct GitProtocol {
    client: Client,
}

#[derive(Debug)]
pub struct RemoteRef {
    pub name: String,
    pub hash: String,
    pub peeled: Option<String>, // 对于带注释的tag
}

#[derive(Debug)]
pub struct PackfileData {
    pub data: Vec<u8>,
    pub refs: Vec<RemoteRef>,
}

impl GitProtocol {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("git/2.0.0 (custom)")
            .build()
            .map_err(|e| GitError::network_error(format!("Failed to create HTTP client: {}", e)))?;
        
        Ok(GitProtocol { client })
    }
    
    /// HTTP(S) Git Smart Protocol 实现
    pub fn fetch_via_http(&self, url: &str, refs_wanted: &[String]) -> Result<PackfileData> {
        // 第一步：获取远程引用列表
        let refs = self.discover_refs_http(url)?;
        
        // 第二步：计算需要的对象
        let wants = self.calculate_wants(&refs, refs_wanted)?;
        
        if wants.is_empty() {
            return Ok(PackfileData {
                data: Vec::new(),
                refs,
            });
        }
        
        // 第三步：请求packfile
        let packfile = self.upload_pack_http(url, &wants)?;
        
        Ok(PackfileData {
            data: packfile,
            refs,
        })
    }
    
    fn discover_refs_http(&self, base_url: &str) -> Result<Vec<RemoteRef>> {
        let url = format!("{}/info/refs?service=git-upload-pack", base_url);
        
        let response = self.client
            .get(&url)
            // 不设置协议版本，使用默认
            .send()
            .map_err(|e| GitError::network_error(format!("Failed to discover refs: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(GitError::network_error(format!(
                "HTTP error {}: {}", 
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }
        
        let body = response.text()
            .map_err(|e| GitError::network_error(format!("Failed to read response: {}", e)))?;
        
        self.parse_refs_response(&body)
    }
    
    fn parse_refs_response(&self, body: &str) -> Result<Vec<RemoteRef>> {
        println!("DEBUG: Parsing refs response, body length: {}", body.len());
        println!("DEBUG: First 200 chars: {:?}", &body[..std::cmp::min(200, body.len())]);
        
        let mut refs: Vec<RemoteRef> = Vec::new();
        
        // 使用 pkt-line 格式解析
        let mut pos = 0;
        let body_bytes = body.as_bytes();
        
        // 跳过第一个服务声明包
        if let Some(first_packet) = self.read_pkt_line(&body_bytes, &mut pos) {
            let first_line = String::from_utf8_lossy(&first_packet);
            println!("DEBUG: First packet: {:?}", first_line);
            if !first_line.contains("git-upload-pack") {
                return Err(GitError::protocol_error("Invalid refs response"));
            }
        }
        
        // 跳过第一个 flush packet（服务声明后的分隔符）
        if let Some(packet_data) = self.read_pkt_line(&body_bytes, &mut pos) {
            if packet_data.is_empty() {
                println!("DEBUG: Skipped first flush packet");
            } else {
                // 如果不是 flush，回退位置并处理
                pos -= 4;
            }
        }
        
        // 读取引用包
        let mut packet_count = 0;
        while pos < body_bytes.len() {
            if let Some(packet_data) = self.read_pkt_line(&body_bytes, &mut pos) {
                packet_count += 1;
                if packet_data.is_empty() {
                    println!("DEBUG: Found final flush packet at packet {}", packet_count);
                break;
            }
            
                let line = String::from_utf8_lossy(&packet_data);
                println!("DEBUG: Packet {}: {:?}", packet_count, line);
                
                // 解析引用行：hash ref_name [capabilities]
                let line = if let Some(null_pos) = line.find('\0') {
                    &line[..null_pos] // 移除能力声明
                } else {
                    &line
                };
                
                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.len() >= 2 {
                    let hash = parts[0].to_string();
                    let ref_name = parts[1].to_string();
                    
                    println!("DEBUG: Found ref: {} -> {}", ref_name, hash);
                    
                    // 处理peeled引用（^{}）
                    if ref_name.ends_with("^{}") {
                        if let Some(last_ref) = refs.last_mut() {
                            last_ref.peeled = Some(hash);
                        }
                    } else {
                    refs.push(RemoteRef {
                            name: ref_name,
                            hash,
                        peeled: None,
                    });
                    }
                }
            } else {
                break;
                }
            }
            
        println!("DEBUG: Total refs found: {}", refs.len());
        for r in &refs {
            println!("DEBUG: Ref: {} -> {}", r.name, r.hash);
        }
        
        Ok(refs)
    }
    
    fn read_pkt_line(&self, data: &[u8], pos: &mut usize) -> Option<Vec<u8>> {
        if *pos + 4 > data.len() {
            return None;
        }
        
        // 读取长度
        let len_bytes = &data[*pos..*pos + 4];
        let len_str = std::str::from_utf8(len_bytes).ok()?;
        let packet_len = u16::from_str_radix(len_str, 16).ok()?;
        
        *pos += 4;
        
        if packet_len == 0 {
            // flush packet
            return Some(Vec::new());
        }
        
        if packet_len < 4 {
            return None;
        }
        
        let content_len = packet_len as usize - 4;
        if *pos + content_len > data.len() {
            return None;
        }
        
        let content = data[*pos..*pos + content_len].to_vec();
        *pos += content_len;
        
        Some(content)
    }
    
    fn calculate_wants(&self, refs: &[RemoteRef], wanted_refs: &[String]) -> Result<Vec<String>> {
        let mut wants = Vec::new();
        
        println!("DEBUG: calculate_wants called with {} refs, {} wanted_refs", refs.len(), wanted_refs.len());
        for r in refs {
            println!("DEBUG: Available ref: {}", r.name);
        }
        
        if wanted_refs.is_empty() {
            // 如果没有指定特定引用，获取所有heads
            for ref_info in refs {
                if ref_info.name.starts_with("refs/heads/") {
                    wants.push(ref_info.hash.clone());
                    println!("DEBUG: Want ref: {} -> {}", ref_info.name, ref_info.hash);
                }
            }
        } else {
            // 获取指定的引用
            for wanted in wanted_refs {
                if let Some(ref_info) = refs.iter().find(|r| r.name == *wanted) {
                    wants.push(ref_info.hash.clone());
                    println!("DEBUG: Want specific ref: {} -> {}", ref_info.name, ref_info.hash);
                }
            }
        }
        
        println!("DEBUG: Total wants: {}", wants.len());
        
        Ok(wants)
    }
    
    fn upload_pack_http(&self, base_url: &str, wants: &[String]) -> Result<Vec<u8>> {
        println!("DEBUG: upload_pack_http called with {} wants", wants.len());
        for want in wants {
            println!("DEBUG: Want: {}", want);
        }
        
        let url = format!("{}/git-upload-pack", base_url);
        println!("DEBUG: POST URL: {}", url);
        
        // 构建upload-pack请求体
        let mut request_body = Vec::new();
        
        // 添加能力和第一个want
        let caps = "multi_ack_detailed side-band-64k thin-pack ofs-delta";
        if !wants.is_empty() {
            let first_want = format!("want {} {}\n", wants[0], caps);
            println!("DEBUG: First want line: {:?}", first_want);
            request_body.extend_from_slice(&self.encode_pkt_line(&first_want));
            
            // 添加其他want行
            for want in &wants[1..] {
                let want_line = format!("want {}\n", want);
                println!("DEBUG: Additional want line: {:?}", want_line);
                request_body.extend_from_slice(&self.encode_pkt_line(&want_line));
            }
        }
        
        // 添加flush包
        request_body.extend_from_slice(b"0000");
        
        // 添加done（表示我们没有对象要提供）
        request_body.extend_from_slice(&self.encode_pkt_line("done\n"));
        
        println!("DEBUG: Request body length: {}", request_body.len());
        println!("DEBUG: Request body: {:?}", String::from_utf8_lossy(&request_body));
        
        let response = self.client
            .post(&url)
            .header("Content-Type", "application/x-git-upload-pack-request")
            .body(request_body)
            .send()
            .map_err(|e| GitError::network_error(format!("Failed to upload-pack: {}", e)))?;
        
        println!("DEBUG: Response status: {}", response.status());
        
        if !response.status().is_success() {
            return Err(GitError::network_error(format!(
                "HTTP error {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }
        
        let body = response.bytes()
            .map_err(|e| GitError::network_error(format!("Failed to read packfile: {}", e)))?;
        
        println!("DEBUG: Response body length: {}", body.len());
        if body.len() > 0 {
            println!("DEBUG: First 100 bytes: {:?}", &body[..std::cmp::min(100, body.len())]);
        }
        
        // 解析响应，提取packfile数据
        self.extract_packfile_from_response(&body)
    }
    
    fn encode_pkt_line(&self, line: &str) -> Vec<u8> {
        let len = line.len() + 4;
        let mut result = format!("{:04x}", len).into_bytes();
        result.extend_from_slice(line.as_bytes());
        result
    }
    
    fn extract_packfile_from_response(&self, response: &[u8]) -> Result<Vec<u8>> {
        let mut pos = 0;
        let mut packfile_data = Vec::new();
        let mut nak_received = false;
        
        while pos < response.len() {
            if pos + 4 > response.len() {
                break;
            }
            
            // 读取包长度
            let len_bytes = &response[pos..pos + 4];
            let len_str = std::str::from_utf8(len_bytes)
                .map_err(|_| GitError::protocol_error("Invalid packet length"))?;
            
            let packet_len = u32::from_str_radix(len_str, 16)
                .map_err(|_| GitError::protocol_error("Invalid packet length format"))?;
            
            if packet_len == 0 {
                // Flush packet
                pos += 4;
                continue;
            }
            
            if pos + packet_len as usize > response.len() {
                break;
            }
            
            let packet_data = &response[pos + 4..pos + packet_len as usize];
            
            // 检查是否是side-band数据
            if !packet_data.is_empty() {
                // 检查是否是NAK消息
                if !nak_received && packet_data.starts_with(b"NAK") {
                    nak_received = true;
                    pos += packet_len as usize;
                    continue;
                }
                
                match packet_data[0] {
                    1 => {
                        // Band 1: packfile data
                        packfile_data.extend_from_slice(&packet_data[1..]);
                    }
                    2 => {
                        // Band 2: progress messages
                        if let Ok(msg) = std::str::from_utf8(&packet_data[1..]) {
                            print!("remote: {}", msg);
                        }
                    }
                    3 => {
                        // Band 3: error messages
                        if let Ok(msg) = std::str::from_utf8(&packet_data[1..]) {
                            return Err(GitError::protocol_error(&format!("Remote error: {}", msg)));
                        }
                    }
                    b'P' => {
                        // 可能是直接的PACK数据 (PACK header)
                        packfile_data.extend_from_slice(packet_data);
                    }
                    _ => {
                        // 其他数据，忽略
                    }
                }
            }
            
            pos += packet_len as usize;
        }
        
        println!("DEBUG: Total packfile data extracted: {} bytes", packfile_data.len());
        if packfile_data.len() >= 8 {
            println!("DEBUG: Packfile header: {:?}", &packfile_data[0..8]);
            if packfile_data.starts_with(b"PACK") {
                println!("DEBUG: Valid PACK header found!");
            } else {
                println!("DEBUG: No PACK header, trying to find it...");
                // 尝试在数据中找到PACK头
                for i in 0..std::cmp::min(1000, packfile_data.len() - 4) {
                    if &packfile_data[i..i+4] == b"PACK" {
                        println!("DEBUG: Found PACK header at offset {}", i);
                        return Ok(packfile_data[i..].to_vec());
                    }
                }
            }
        }
        
        Ok(packfile_data)
    }
}
