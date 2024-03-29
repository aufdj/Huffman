use std::{
    collections::{BinaryHeap, HashMap},
    io::{BufReader, BufWriter, BufRead},
    fs::File,
};

use crate::{
    Node, NodeType,
    BufferedRead, BufferedWrite, BufferState,
};

pub fn decompress(file_in: &mut BufReader<File>, file_out: &mut BufWriter<File>, file_in_size: u64) {
    let padding = file_in.read_byte();

    let mut frequencies = [0u32; 256];

    for i in 0..256 {
        frequencies[i] = file_in.read_u32();    
    }

    let mut heap = BinaryHeap::with_capacity(512);
    for i in 0..256 {                                               
        heap.push(                                                  
            Node::new(
                frequencies[i],
                NodeType::Leaf(i as u8)
            )
        );
    }   

    build_tree(&mut heap); 

    let mut codes = HuffmanCodeMap::new();
    gen_codes(heap.peek().unwrap(), vec![], &mut codes);

    let mut curr_code: Vec<u8> = Vec::with_capacity(8);
    let mut pos = 1026;
    file_in.fill_buf().unwrap(); // Don't use fill_buffer() here
    
    loop {
        for byte in file_in.buffer().iter() {
            if pos >= file_in_size {
                for j in (0..=(7 - padding)).rev() {
                    curr_code.push((*byte >> j) & 1);
                    if let Some(byte) = codes.get(&curr_code) {
                        file_out.write_byte(*byte);
                        curr_code.clear();
                    }
                }
            } 
            else {
                for j in (0..=7).rev() {
                    curr_code.push((*byte >> j) & 1);
                    if let Some(byte) = codes.get(&curr_code) {
                        file_out.write_byte(*byte);
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

type HuffmanCodeMap = HashMap<Vec<u8>, u8>;

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
            codes.insert(prefix, byte);
        }
    }
}

fn build_tree(heap: &mut BinaryHeap<Node>) {
    while heap.len() > 1 {
        let left_child = heap.pop().unwrap();
        let right_child = heap.pop().unwrap();
        heap.push(
            Node::new(
                left_child.frequency + right_child.frequency, 
                NodeType::Internal(
                    Box::new(left_child), 
                    Box::new(right_child)
                )
            )
        );
    }
}