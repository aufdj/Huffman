mod encoder;
mod decoder;

use std::{
    fs::{File, metadata},
    io::{Read, Write, BufReader, BufWriter, BufRead},
    time::Instant,
    path::Path,
    cmp::Ordering,
};

use crate::{
    encoder::compress,
    decoder::decompress,
};

// Indicates an empty or non-empty buffer. 
#[derive(PartialEq, Eq)]
pub enum BufferState {
    NotEmpty,
    Empty,
}

/// A trait for handling buffered reading.
pub trait BufferedRead {
    fn read_byte(&mut self) -> u8;
    fn read_u32(&mut self) -> u32;
    fn read_u64(&mut self) -> u64;
    fn fill_buffer(&mut self) -> BufferState;
}
impl BufferedRead for BufReader<File> {
    /// Read one byte from an input file.
    fn read_byte(&mut self) -> u8 {
        let mut byte = [0u8; 1];

        if let Ok(_) = self.read(&mut byte) {
            if self.buffer().is_empty() {
                self.consume(self.capacity());

                if let Err(e) = self.fill_buf() {
                    println!("Function read_byte failed.");
                    println!("Error: {}", e);
                }
            }
        }
        else {
            println!("Function read_byte failed.");
        }
        u8::from_le_bytes(byte)
    }
    fn read_u32(&mut self) -> u32 {
        let mut bytes = [0u8; 4];

        if let Ok(len) = self.read(&mut bytes) {
            if self.buffer().is_empty() {
                self.consume(self.capacity());

                if let Err(e) = self.fill_buf() {
                    println!("Function read_u32 failed.");
                    println!("Error: {}", e);
                }
                if len < 4 {
                    self.read_exact(&mut bytes[len..]).unwrap();
                }
            }
        }
        else {
            println!("Function read_u32 failed.");
        }
        u32::from_le_bytes(bytes)
    }
    /// Read 8 bytes from an input file, taking care to handle reading 
    /// across buffer boundaries.
    fn read_u64(&mut self) -> u64 {
        let mut bytes = [0u8; 8];

        if let Ok(len) = self.read(&mut bytes) {
            if self.buffer().is_empty() {
                self.consume(self.capacity());
                
                if let Err(e) = self.fill_buf() {
                    println!("Function read_u64 failed.");
                    println!("Error: {}", e);
                }
                if len < 8 {
                    self.read_exact(&mut bytes[len..]).unwrap();
                }
            }
        }
        else {
            println!("Function read_u64 failed.");
        }
        u64::from_le_bytes(bytes)
    }
    /// Fills the input buffer, returning the buffer's state.
    fn fill_buffer(&mut self) -> BufferState {
        self.consume(self.capacity());
        if let Err(e) = self.fill_buf() {
            println!("Function fill_buffer failed.");
            println!("Error: {}", e);
        }
        if self.buffer().is_empty() {
            return BufferState::Empty;
        }
        BufferState::NotEmpty
    }
}

/// A trait for handling buffered writing.
pub trait BufferedWrite {
    fn write_byte(&mut self, output: u8);
    fn write_u32(&mut self, output: u32);
    fn write_u64(&mut self, output: u64);
    fn flush_buffer(&mut self);
}
impl BufferedWrite for BufWriter<File> {
    /// Write one byte to an output file.
    fn write_byte(&mut self, output: u8) {
        if let Err(e) = self.write(&[output]) {
            println!("Function write_byte failed.");
            println!("Error: {}", e);
        }
        
        if self.buffer().len() >= self.capacity() {
            if let Err(e) = self.flush() {
                println!("Function write_byte failed.");
                println!("Error: {}", e);
            }
        }
    }
    fn write_u32(&mut self, output: u32) {
        if let Err(e) = self.write(&output.to_le_bytes()[..]) {
            println!("Function write_32 failed.");
            println!("Error: {}", e);
        }
        
        if self.buffer().len() >= self.capacity() {
            if let Err(e) = self.flush() {
                println!("Function write_32 failed.");
                println!("Error: {}", e);
            }
        }
    }
    /// Write 8 bytes to an output file.
    fn write_u64(&mut self, output: u64) {
        if let Err(e) = self.write(&output.to_le_bytes()[..]) {
            println!("Function write_u64 failed.");
            println!("Error: {}", e);
        }
        
        if self.buffer().len() >= self.capacity() {
            if let Err(e) = self.flush() {
                println!("Function write_u64 failed.");
                println!("Error: {}", e);
            }
        }
    }
    /// Flush buffer to file.
    fn flush_buffer(&mut self) {
        if let Err(e) = self.flush() {
            println!("Function flush_buffer failed.");
            println!("Error: {}", e);
        }    
    }
}


/// Takes a file path and returns an input file wrapped in a BufReader.
pub fn new_input_file(capacity: usize, path: &Path) -> BufReader<File> {
    BufReader::with_capacity(
        capacity, File::open(path).unwrap()
    )
}

/// Takes a file path and returns an output file wrapped in a BufWriter.
pub fn new_output_file(capacity: usize, path: &Path) -> BufWriter<File> {
    BufWriter::with_capacity(
        capacity, File::create(path).unwrap()
    )
}

// Node implementation ---------------------------------------------
#[derive(Eq, PartialEq)]
pub enum NodeType {
    Internal(Box<Node>, Box<Node>),
    Leaf(u8),
}
#[derive(Eq, PartialEq)]
pub struct Node {
    frequency: u32,
    node_type: NodeType,
}
impl Node {
    pub fn new(frequency: u32, node_type: NodeType) -> Node {
        Node { 
            frequency, 
            node_type 
        }
    }
}
impl Ord for Node {
    fn cmp(&self, rhs: &Self) -> Ordering {
        rhs.frequency.cmp(&self.frequency)
    }
} 
impl PartialOrd for Node {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(&rhs))
    }
}
// -----------------------------------------------------------------

fn main() {
    let start_time = Instant::now();
    let args = std::env::args().skip(1).collect::<Vec<String>>();
    let mut file_in  = new_input_file(4096, Path::new(&args[1]));
    let mut file_out = new_output_file(4096, Path::new(&args[2]));

    match (&args[0]).as_str() {
        "c" => { 
            let file_in_size = file_len(Path::new(&args[1]));
            compress(&mut file_in, &mut file_out);
            let file_out_size = file_len(Path::new(&args[2]));

            println!("{} bytes -> {} bytes in {:.2?}", 
                file_in_size, file_out_size, start_time.elapsed());   
        },
        "d" => { 
            let file_in_size = file_len(Path::new(&args[1]));
            decompress(&mut file_in, &mut file_out, file_in_size);
            let file_out_size = file_len(Path::new(&args[2])); 

            println!("{} bytes -> {} bytes in {:.2?}", 
                file_in_size, file_out_size, start_time.elapsed());   
        }, 
        _ => {
            println!("To compress: c input output");
            println!("To decompress: d input output");
        }
    }    
}

fn file_len(path: &Path) -> u64 {
    metadata(path).unwrap().len()
}

