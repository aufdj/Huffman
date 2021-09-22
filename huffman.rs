use std::collections::BinaryHeap;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write, BufReader, BufWriter, BufRead, Seek};
use std::env;
use std::fs::metadata;
use std::time::Instant;
use std::path::Path;

// Convenience functions for buffered IO ---------------------------
fn write(file_out: &mut BufWriter<File>, output: u8) {
    file_out.write(&[output]).unwrap();
    if file_out.buffer().len() >= file_out.capacity() { 
        file_out.flush().unwrap(); 
    }
}
fn read(buf_in: &mut BufReader<File>, input: &mut [u8; 4]) -> usize {
    let bytes_read = buf_in.read(input).unwrap();
    if buf_in.buffer().len() <= 0 { 
        buf_in.consume(buf_in.capacity()); 
        buf_in.fill_buf().unwrap();
    }
    bytes_read
}
// -----------------------------------------------------------------

// Node implementation ---------------------------------------------
#[derive(Eq, PartialEq)]
enum NodeType {
    Internal(Box<Node>, Box<Node>),
    Leaf(u8),
}
#[derive(Eq, PartialEq)]
struct Node {
    frequency: u32,
    node_type: NodeType,
}
impl Node {
    fn new(frequency: u32, node_type: NodeType) -> Node {
        Node { frequency, node_type }
    }
}
impl Ord for Node {
    fn cmp(&self, rhs: &Self) -> std::cmp::Ordering {
        rhs.frequency.cmp(&self.frequency)
    }
} 
impl PartialOrd for Node {
    fn partial_cmp(&self, rhs: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(&rhs))
    }
}
// -----------------------------------------------------------------

// Model data to get frequency distribution ------------------------
fn model(file_in: &mut BufReader<File>) -> [u32; 256] {
    file_in.fill_buf().unwrap();
    let mut frequencies = [1u32; 256];
    loop {
        for i in 0..file_in.buffer().len() {
            frequencies[file_in.buffer()[i as usize] as usize] += 1;
        }
        file_in.consume(file_in.capacity());
        file_in.fill_buf().unwrap();
        if file_in.buffer().is_empty() {
            break;
        }
    }
    frequencies
}
// -----------------------------------------------------------------

// Build tree from leaf nodes --------------------------------------
fn build_tree(heap: &mut BinaryHeap<Node>) {
    while heap.len() > 1 {
        let left_child = heap.pop().unwrap();
        let right_child = heap.pop().unwrap();
        heap.push(
            Node::new(
                left_child.frequency + right_child.frequency, 
                NodeType::Internal(Box::new(left_child), Box::new(right_child))
            )
        );
    }
}
// -----------------------------------------------------------------

// Walk down every branch of tree to get codes for every byte ------
type HuffmanCodeMap  = BTreeMap<u8, Vec<u8>>;
type HuffmanCodeMapD = BTreeMap<Vec<u8>, u8>;

fn generate_codes(node: &Node, prefix: Vec<u8>, codes: &mut HuffmanCodeMap) {
    match node.node_type {
        NodeType::Internal(ref left_child, ref right_child) => {
            let mut left_prefix = prefix.clone();
            left_prefix.push(0);
            generate_codes(&left_child, left_prefix, codes);

            let mut right_prefix = prefix;
            right_prefix.push(1);
            generate_codes(&right_child, right_prefix, codes);
        }
        NodeType::Leaf(byte) => {
            codes.insert(byte, prefix);
        }
    }
}
fn generate_codes_d(node: &Node, prefix: Vec<u8>, codes: &mut HuffmanCodeMapD) {
    match node.node_type {
        NodeType::Internal(ref left_child, ref right_child) => {
            let mut left_prefix = prefix.clone();
            left_prefix.push(0);
            generate_codes_d(&left_child, left_prefix, codes);

            let mut right_prefix = prefix;
            right_prefix.push(1);
            generate_codes_d(&right_child, right_prefix, codes);
        }
        NodeType::Leaf(byte) => {
            codes.insert(prefix, byte);
        }
    }
}
// -----------------------------------------------------------------

fn compress(file_in: &mut BufReader<File>, file_out: &mut BufWriter<File>) {
    write(file_out, 0);

    let frequencies: [u32; 256] = model(file_in);                   // model data to get prob. distribution

    for i in 0..256 {                                               // include model as compressed data header
        for j in 0..4 {                                             //
            write(file_out,                                         //
                ((frequencies[i] >> (j*8)) & 0xFF) as u8);          //
        }                                                           //
    }                                                               //
    
    let mut heap: BinaryHeap<Node> = BinaryHeap::new();
    for i in 0..256 {                                               // add leaf nodes to heap
        heap.push(                                                  //
            Node::new(frequencies[i], NodeType::Leaf(i as u8))      //
        );                                                          //
    }                                                               //

    build_tree(&mut heap);       

    let mut codes = HuffmanCodeMap::new();                          // walk down tree and generate codes
    generate_codes(heap.peek().unwrap(), vec![0u8; 0], &mut codes); // 

    file_in.get_ref().rewind().unwrap();
    file_in.fill_buf().unwrap();

    let mut packed_codes: u8 = 0;
    let mut num_bits: u8 = 0;
    loop {
        for i in 0..file_in.buffer().len() { 
            // Get huffman code corresponding to current byte and write bits to output
            for bit in codes.get(&file_in.buffer()[i]).unwrap() {
                if num_bits >= 8 {
                    write(file_out, packed_codes);
                    packed_codes = 0;
                    num_bits = 0;
                }
                packed_codes += packed_codes + bit;
                num_bits += 1;
            }
        }
        
        file_in.consume(file_in.capacity());
        file_in.fill_buf().unwrap();
        if file_in.buffer().is_empty() {
            if num_bits > 0 {
                write(file_out, packed_codes);
            }
            file_out.flush().unwrap();

            file_out.get_ref().rewind().unwrap();
            write(file_out, 8 - num_bits);
            file_out.flush().unwrap();

            println!("Finished Compressing.");
            break;
        }
    }  
}

fn decompress(file_in: &mut BufReader<File>, file_out: &mut BufWriter<File>, file_in_size: u64) {
    let mut padding = [0u8; 1];
    file_in.read(&mut padding).unwrap();

    let mut frequencies = [0u32; 256];
    let mut frequency = [0; 4];

    for i in 0..256 {
        read(file_in, &mut frequency);
        frequencies[i] = (frequency[0] as u32)        + ((frequency[1] as u32) << 8)  
                      + ((frequency[2] as u32) << 16) + ((frequency[3] as u32) << 24);    
    }

    let mut heap: BinaryHeap<Node> = BinaryHeap::new();
    for i in 0..256 {                                               
        heap.push(                                                  
            Node::new(frequencies[i], NodeType::Leaf(i as u8))     
        );                                                          
    }   

    build_tree(&mut heap); 

    let mut codes = HuffmanCodeMapD::new();
    generate_codes_d(heap.peek().unwrap(), vec![0u8; 0], &mut codes);

    file_in.fill_buf().unwrap();
    let mut current_code: Vec<u8> = Vec::new();

    let mut pos = 1026;

    loop {
        for i in 0..file_in.buffer().len() {
            if pos >= file_in_size { 
                for j in (0..=(7 - padding[0])).rev() {
                    current_code.push((file_in.buffer()[i] >> j) & 1);
                    if codes.contains_key(&current_code) {
                        write(file_out, *codes.get(&current_code).unwrap());
                        current_code.clear();
                    }
                }
            } else {
                for j in (0..=7).rev() {
                    current_code.push((file_in.buffer()[i] >> j) & 1);
                    if codes.contains_key(&current_code) {
                        write(file_out, *codes.get(&current_code).unwrap());
                        current_code.clear();
                    }
                }
            }
            pos += 1;
        }
        file_in.consume(file_in.capacity());
        file_in.fill_buf().unwrap();
        if file_in.buffer().is_empty() {
            file_out.flush().unwrap();
            println!("Finished Decompressing.");
            break;
        }
    }   
}

fn main() {
    let start_time = Instant::now();
    let args: Vec<String> = env::args().collect();
    let mut file_in  = BufReader::with_capacity(4096, File::open(&args[2]).unwrap());
    let mut file_out = BufWriter::with_capacity(4096, File::create(&args[3]).unwrap());

    match (&args[1]).as_str() {
        "c" => { 
            let file_in_size = metadata(Path::new(&args[2])).unwrap().len();
            compress(&mut file_in, &mut file_out);
            let file_out_size = metadata(Path::new(&args[3])).unwrap().len();  
            println!("{} bytes -> {} bytes in {:.2?}", file_in_size, file_out_size, start_time.elapsed());   
        },
        "d" => { 
            let file_in_size = metadata(Path::new(&args[2])).unwrap().len();
            decompress(&mut file_in, &mut file_out, file_in_size);
            let file_out_size = metadata(Path::new(&args[3])).unwrap().len();  
            println!("{} bytes -> {} bytes in {:.2?}", file_in_size, file_out_size, start_time.elapsed());   
        }, 
        _ => {
            println!("To compress: c input output");
            println!("To decompress: d input output");
        }
    }    
}
