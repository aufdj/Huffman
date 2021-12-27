use std::{
    collections::{BinaryHeap, BTreeMap},
    fs::{File, metadata},
    io::{Read, Write, BufReader, BufWriter, BufRead, Seek},
    env,
    time::Instant,
    path::Path,
    cmp::Ordering,
};

// Convenience functions for buffered I/O ---------------------------------------------------------- Convenience functions for buffered I/O
#[derive(PartialEq, Eq)]
enum BufferState {
    NotEmpty,
    Empty,
}

trait BufferedRead {
    fn read_byte(&mut self, input: &mut [u8; 1]);
    fn read_u32(&mut self, input: &mut [u8; 4]);
    fn fill_buffer(&mut self) -> BufferState;
}
impl BufferedRead for BufReader<File> {
    fn read_byte(&mut self, input: &mut [u8; 1]) {
        match self.read(input) {
            Ok(_)  => {},
            Err(e) => { 
                println!("Function read_byte failed."); 
                println!("Error: {}", e);
            },
        };
        if self.buffer().is_empty() { 
            self.consume(self.capacity()); 
            match self.fill_buf() {
                Ok(_)  => {},
                Err(e) => {
                    println!("Function read_byte failed.");
                    println!("Error: {}", e);
                },
            }
        }
    }
    fn read_u32(&mut self, input: &mut [u8; 4]) {
        match self.read(input) {
            Ok(_)  => {},
            Err(e) => { 
                println!("Function read_u32 failed."); 
                println!("Error: {}", e);
            },
        };
        if self.buffer().is_empty() { 
            self.consume(self.capacity()); 
            match self.fill_buf() {
                Ok(_)  => {},
                Err(e) => { 
                    println!("Function read_u32 failed."); 
                    println!("Error: {}", e);
                },
            }
        }
    }
    fn fill_buffer(&mut self) -> BufferState {
        self.consume(self.capacity());
        match self.fill_buf() {
            Ok(_)  => {},
            Err(e) => { 
                println!("Function fill_buffer failed."); 
                println!("Error: {}", e);
            },
        }
        if self.buffer().is_empty() { 
            return BufferState::Empty; 
        }
        BufferState::NotEmpty
    }
}
trait BufferedWrite {
    fn write_byte(&mut self, output: u8);
    fn write_u32(&mut self, output: u32);
    fn flush_buffer(&mut self);
}
impl BufferedWrite for BufWriter<File> {
    fn write_byte(&mut self, output: u8) {
        match self.write(&[output]) {
            Ok(_)  => {},
            Err(e) => { 
                println!("Function write_byte failed."); 
                println!("Error: {}", e);
            },
        }
        if self.buffer().len() >= self.capacity() { 
            match self.flush() {
                Ok(_)  => {},
                Err(e) => { 
                    println!("Function write_byte failed."); 
                    println!("Error: {}", e);
                },
            } 
        }
    }
    fn write_u32(&mut self, output: u32) {
        match self.write(&output.to_le_bytes()[..]) {
            Ok(_)  => {},
            Err(e) => { 
                println!("Function write_usize failed."); 
                println!("Error: {}", e);
            },
        }
        if self.buffer().len() >= self.capacity() { 
            match self.flush() {
                Ok(_)  => {},
                Err(e) => { 
                    println!("Function write_usize failed."); 
                    println!("Error: {}", e);
                },
            } 
        }
    }
    fn flush_buffer(&mut self) {
        match self.flush() {
            Ok(_)  => {},
            Err(e) => { 
                println!("Function flush_buffer failed."); 
                println!("Error: {}", e);
            },
        }    
    }
}
fn new_input_file(capacity: usize, file_name: &str) -> BufReader<File> {
    BufReader::with_capacity(capacity, File::open(file_name).unwrap())
}
fn new_output_file(capacity: usize, file_name: &str) -> BufWriter<File> {
    BufWriter::with_capacity(capacity, File::create(file_name).unwrap())
}
// ----------------------------------------------------------------------------------------------------------------------------------------


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

// Model data to get frequency distribution ------------------------
fn model(file_in: &mut BufReader<File>) -> [u32; 256] {
    file_in.fill_buffer();
    let mut frequencies = [1u32; 256];
    loop {
        for byte in file_in.buffer().iter() {
            frequencies[*byte as usize] += 1;
        }
        if file_in.fill_buffer() == BufferState::Empty { break; }
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

fn gen_codes(node: &Node, prefix: Vec<u8>, codes: &mut HuffmanCodeMap) {
    match node.node_type {
        NodeType::Internal(ref left_child, ref right_child) => {
            let mut left_prefix = prefix.clone();
            left_prefix.push(0);
            gen_codes(&left_child, left_prefix, codes);

            let mut right_prefix = prefix;
            right_prefix.push(1);
            gen_codes(&right_child, right_prefix, codes);
        }
        NodeType::Leaf(byte) => {
            codes.insert(byte, prefix);
        }
    }
}
fn gen_codes_d(node: &Node, prefix: Vec<u8>, codes: &mut HuffmanCodeMapD) {
    match node.node_type {
        NodeType::Internal(ref left_child, ref right_child) => {
            let mut left_prefix = prefix.clone();
            left_prefix.push(0);
            gen_codes_d(&left_child, left_prefix, codes);

            let mut right_prefix = prefix;
            right_prefix.push(1);
            gen_codes_d(&right_child, right_prefix, codes);
        }
        NodeType::Leaf(byte) => {
            codes.insert(prefix, byte);
        }
    }
}
// -----------------------------------------------------------------

fn compress(file_in: &mut BufReader<File>, file_out: &mut BufWriter<File>) {
    file_out.write_byte(0);

    let frequencies: [u32; 256] = model(file_in);                   // Model data to get prob. distribution
     
    for freq in frequencies.iter() {                                // Include model as compressed data header
        file_out.write_u32(*freq);                                  //                                                               
    }                                                               //
    
    let mut heap: BinaryHeap<Node> = BinaryHeap::new();
    for i in 0..256 {                                               // Add leaf nodes to heap
        heap.push(                                                  //
            Node::new(frequencies[i], NodeType::Leaf(i as u8))      //
        );                                                          //
    }                                                               //

    build_tree(&mut heap);       

    let mut codes = HuffmanCodeMap::new();                          // Walk down tree and generate codes
    gen_codes(heap.peek().unwrap(), vec![0u8; 0], &mut codes);      // 

    file_in.rewind().unwrap();
    file_in.fill_buffer();

    let mut packed_codes: u8 = 0;
    let mut num_bits: u8 = 0;
    loop {
        for byte in file_in.buffer().iter() { 
            // Get huffman code corresponding to current byte and write bits to output
            for bit in codes.get(byte).unwrap() {
                if num_bits >= 8 {
                    file_out.write_byte(packed_codes);
                    packed_codes = 0;
                    num_bits = 0;
                }
                packed_codes += packed_codes + bit;
                num_bits += 1;
            }
        }
        
        if file_in.fill_buffer() == BufferState::Empty {
            if num_bits > 0 {
                file_out.write_byte(packed_codes);
            }
            file_out.flush_buffer();
            file_out.rewind().unwrap();
            file_out.write_byte(8 - num_bits);
            file_out.flush_buffer();
            println!("Finished Compressing.");
            break;
        }
    }  
}

fn decompress(file_in: &mut BufReader<File>, file_out: &mut BufWriter<File>, file_in_size: u64) {
    let mut padding = [0u8; 1];
    file_in.read_byte(&mut padding);

    let mut frequencies = [0u32; 256];
    let mut frequency = [0; 4];

    for i in 0..256 {
        file_in.read_u32(&mut frequency);
        frequencies[i] = u32::from_le_bytes(frequency);    
    }

    let mut heap: BinaryHeap<Node> = BinaryHeap::with_capacity(512);
    for i in 0..256 {                                               
        heap.push(                                                  
            Node::new(frequencies[i], NodeType::Leaf(i as u8))     
        );                                                          
    }   

    build_tree(&mut heap); 

    let mut codes = HuffmanCodeMapD::new();
    gen_codes_d(heap.peek().unwrap(), vec![0u8; 0], &mut codes);

    let mut curr_code: Vec<u8> = Vec::with_capacity(8);
    let mut pos = 1026;
    file_in.fill_buf().unwrap(); // Don't use fill_buffer() here 
    
    loop {
        for byte in file_in.buffer().iter() {
            if pos >= file_in_size { 
                for j in (0..=(7 - padding[0])).rev() {
                    curr_code.push((*byte >> j) & 1);
                    if codes.contains_key(&curr_code) {
                        file_out.write_byte(*codes.get(&curr_code).unwrap());
                        curr_code.clear();
                    }
                }
            } 
            else {
                for j in (0..=7).rev() {
                    curr_code.push((*byte >> j) & 1);
                    if codes.contains_key(&curr_code) {
                        file_out.write_byte(*codes.get(&curr_code).unwrap());
                        curr_code.clear();
                    }
                }
            }
            pos += 1;
        }
        if file_in.fill_buffer() == BufferState::Empty {
            file_out.flush_buffer();
            println!("Finished Decompressing.");
            break;
        }
    }   
}

fn main() {
    let start_time = Instant::now();
    let args: Vec<String> = env::args().collect();
    let mut file_in  = new_input_file(4096, &args[2]);
    let mut file_out = new_output_file(4096, &args[3]);

    match (&args[1]).as_str() {
        "c" => { 
            let file_in_size = metadata(Path::new(&args[2])).unwrap().len();
            compress(&mut file_in, &mut file_out);
            let file_out_size = metadata(Path::new(&args[3])).unwrap().len();  
            println!("{} bytes -> {} bytes in {:.2?}", 
            file_in_size, file_out_size, start_time.elapsed());   
        },
        "d" => { 
            let file_in_size = metadata(Path::new(&args[2])).unwrap().len();
            decompress(&mut file_in, &mut file_out, file_in_size);
            let file_out_size = metadata(Path::new(&args[3])).unwrap().len();  
            println!("{} bytes -> {} bytes in {:.2?}", 
            file_in_size, file_out_size, start_time.elapsed());   
        }, 
        _ => {
            println!("To compress: c input output");
            println!("To decompress: d input output");
        }
    }    
}

