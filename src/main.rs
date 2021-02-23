use std::{
    fs::{self, File},
    io::{self, stdin, BufRead, Read, Write},
};

/*
bitmap constants to
*/

fn main() {
    create_index_file("IndexFile.txt");
}

/*
Magic file using latmanshashing to create quick lookup
Will be 30*30*30 combinations as all possible 3 letter combnations are (a-ö+_)^3
Assuming Index file is passed as stdin
*/
fn generate_magic_file(path: &str) {
    let stdin = stdin();
    let mut prefix_i = 0;
    let mut new_offset = 0;
    let mut new_prefix = true;
    let mut prefix: [u8; 3] = [0; 3];
    let mut file = File::create(path).unwrap();
    for (offset, byte) in stdin.lock().bytes().map(|_b| _b.unwrap()).enumerate() {
        if prefix_i == 0 {
            new_offset = offset;
        }
        if prefix_i < 3 {
            if prefix[prefix_i] != byte {
                new_prefix = true;
                if byte == b' ' {
                    for u in &mut prefix[prefix_i..] {
                        *u = byte
                    }
                    prefix_i = 3;
                } else {
                    prefix[prefix_i] = byte;
                }
            }
            prefix_i += 1;
        } else if byte == b'\n' || byte == b'\r' {
            if new_prefix {
                file.write_all(&[b'\n']).unwrap(); //REMEMBER FIRST ROW WILL BE EMPTY!
                let s: String = prefix.iter().map(|&c| c as char).collect();
                file.write_all(s.as_bytes()).unwrap();
            }
            file.write_all(format!("{}", new_offset).as_bytes())
                .unwrap(); //Offset will be recorded as be_bytes ([u8;8?- due to usize])
            file.flush().unwrap();

            new_prefix = false;
            prefix_i = 0;
        }
    }
}

/*
Simple mapping/hash returning where too look in magic file
*/
fn hash() {}

/*
Function to convert token file with repeating words to a "single word"-"single occurence" file
*/
fn create_index_file(path: &str) {
    let stdin = stdin();
    let mut word = String::new();
    let mut file = File::create(path).unwrap();

    for line in stdin.lock().lines().map(|_line| _line.unwrap()) {
        let line = line.trim();
        let words: Vec<&str> = line.split_ascii_whitespace().collect();
        if &word != words.get(0).unwrap() {
            file.write_all(&[b'\n']).unwrap();
            file.flush().unwrap();
            word = words.get(0).unwrap().to_string();
            file.write_all(format!("{} {}", word, words.get(1).unwrap()).as_bytes())
                .unwrap();
        } else {
            file.write_all(format!(" {}", words.get(1).unwrap()).as_bytes())
                .unwrap();
        }
    }
    file.flush().unwrap();
}
