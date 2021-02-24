use std::{
    fs::{self, File},
    io::{self, stdin, BufRead, BufReader, Error, ErrorKind, Read, Seek, SeekFrom, Write},
};

use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
/*
bitmap constants to
*/

const PREVIEW_LEN: usize = 60;
const MAX_PREVIEWS: i64 = 10;

fn main() {
    //generate_magic_file("magic.txt");
    //create_index_file("IndexFile.txt");
    let stdin = stdin();
    loop {
        let mut input = String::new();
        println!("Enter word to search");
        stdin
            .read_line(&mut input)
            .expect("Did not enter a correct string");
        lookup(&input.trim()).expect("Oops!");
    }
}

fn lookup(word_s: &str) -> io::Result<()> {
    let mut korpus = File::open("korpus")?;
    let mut index_file = File::open("IndexFile.txt");
    if index_file.is_err() {
        create_index_file("IndexFile.txt");
        index_file = File::open("IndexFile.txt");
    }
    let mut index_file = BufReader::new(index_file?);
    let mut magic_file = File::open("magic.txt");
    if magic_file.is_err() {
        generate_magic_file("magic.txt");
        magic_file = File::open("magic.txt");
    }
    let mut magic_file = magic_file?;
    //Lookup in magic file!
    let w = WINDOWS_1252.encode(&word_s).0;
    let mut word = [32; 3];
    for i in 0..w.len().min(3) {
        word[i] = w[i];
    }
    let hash = hash(&word[..3]);
    let mut buff_word = [0; 8];
    magic_file.seek(SeekFrom::Start(hash))?;
    magic_file.read_exact(&mut buff_word)?;
    let mut buff_next = [0; 8];
    loop {
        magic_file.read_exact(&mut buff_next)?;
        if buff_next != [32; 8] {
            break;
        }
    }
    //Lookup in index_file!
    let word_offset = u64::from_ne_bytes(buff_word);
    let next_offset = u64::from_ne_bytes(buff_next);
    let result = check_word(word_offset, next_offset, &mut index_file, &word_s);
    if let Ok(indicies) = result {
        let mut occurences = Vec::new();
        let mut buff_sentence = [0; PREVIEW_LEN]; //30 characters long preview
        for i in indicies {
            if i > (PREVIEW_LEN / 2) as u64 {
                korpus.seek(std::io::SeekFrom::Start(
                    (i as i64 - (PREVIEW_LEN / 2) as i64) as u64,
                ))?;
            } else {
                korpus.seek(std::io::SeekFrom::Start(i))?;
            }
            korpus.read_exact(&mut buff_sentence)?;
            occurences.push(buff_sentence);
        }
        let mut count = 0;
        let mut long_count = 0;
        let total = occurences.len();
        let mut input = String::new();
        for passage in occurences {
            let sentence = WINDOWS_1252.decode(&passage).0.replace('\n', " ");
            println!("...{}...\n", sentence);
            long_count += 1;
            count += 1;
            if count >= MAX_PREVIEWS {
                println!("{} of {} See more? y/n", long_count, total);
                loop {
                    stdin()
                        .read_line(&mut input)
                        .expect("Did not enter a correct string");
                    if input == "y\n" {
                        count = 0;
                        input = String::new();
                        break;
                    } else if input == "n\n" {
                        return Ok(());
                    } else {
                        println!("{} Enter y/n", input);
                    }
                }
            }
        }
        return Ok(());
    } else {
        println!("{}", result.unwrap_err().to_string());
    }
    Err(Error::new(ErrorKind::Other, "Error in lookup"))
}
fn check_word(
    mut word_offset: u64,
    mut next_offset: u64,
    i_file: &mut BufReader<File>,
    word: &str,
) -> io::Result<Vec<u64>> {
    let mut read_word = Vec::with_capacity(10);
    let mut false_line = Vec::with_capacity(50);
    //Seems less efficent than just reading
    // while next_offset - word_offset > 1000 {
    //     let mid = (next_offset - word_offset) / 2;
    //     i_file.seek(SeekFrom::Start(mid))?;
    //     i_file.read_until(b'\n', &mut false_line)?;

    //     i_file.read_until(b' ', &mut read_word)?;
    //     read_word.pop(); //remove ' '
    //     for i in 3..read_word.len() {
    //         if let Some(c_w) = word.get(i) {
    //             if let Some(c_c) = read_word.get(i) {
    //                 match c_c.cmp(c_w) {
    //                     std::cmp::Ordering::Less => {
    //                         word_offset = mid;
    //                     }
    //                     std::cmp::Ordering::Equal => {
    //                         continue;
    //                     }
    //                     std::cmp::Ordering::Greater => {
    //                         next_offset = mid;
    //                     }
    //                 }
    //             } else {
    //                 word_offset = mid;
    //                 break;
    //             }
    //         } else if read_word.get(i).is_some() {
    //             next_offset = mid;
    //             break;
    //         } else {
    //             break; //One should never come here
    //         }
    //     }
    // }
    let mut result = String::new();
    i_file.seek(SeekFrom::Start(word_offset))?;
    loop {
        if word_offset > next_offset {
            return Err(Error::new(ErrorKind::Other, "Word not found"));
        }
        i_file.read_until(b' ', &mut read_word)?;
        read_word.pop(); //remove ' '
        if read_word == WINDOWS_1252.encode(word).0.to_vec() {
            i_file.read_line(&mut result)?;
            let results: Vec<u64> = result
                .split_whitespace()
                .map(|s| s.parse::<u64>().unwrap())
                .collect();
            return Ok(results);
        }
        i_file.read_until(b'\n', &mut false_line)?;
        read_word.clear();
        word_offset = i_file.seek(SeekFrom::Current(0))?;
    }
}

/*
Simple mapping/hash returning where too look in magic file
Assumes Latin 1 encoding
*/
fn hash(word: &[u8]) -> u64 {
    let mut index = 0;
    for i in 0..word.len() {
        if i > 2 {
            break;
        }
        match word[i] {
            228 => index += 27 * 30u64.pow((2 - i) as u32), //ä
            229 => index += 28 * 30u64.pow((2 - i) as u32), //å
            246 => index += 29 * 30u64.pow((2 - i) as u32), //ö
            32 => {}                                        //' '
            c => index += (c - 96) as u64 * 30u64.pow((2 - i) as u32),
        }
    }
    if index != 0 {
        index -= 900; //Remove initial 30^2 from a__
    }
    index * 8
}

/*
Magic file using latmanshashing to create quick lookup
Will be 30*30*30*8 combinations as all possible 3 letter combnations are (a-ö+_)^3
Assuming Index file is passed as stdin
*/
fn generate_magic_file(path: &str) {
    let stdin = stdin();
    let mut prefix_i = 0;
    let mut new_offset = 0;
    let mut new_prefix = false;
    let mut prefix: [u8; 3] = [0; 3];
    let mut file_offset = 0;
    let mut file = File::create(path).unwrap();

    for (offset, byte) in File::open("IndexFile.txt")
        .expect("You must have a valid IndexFile.txt file in same file as src/")
        .bytes()
        .map(|_b| _b.unwrap())
        .enumerate()
    {
        if byte == b'\n' || byte == b'\r' {
            if new_prefix {
                let pos = file.seek(SeekFrom::Current(0)).unwrap();
                file.write_all(&vec![b' '; (hash(&prefix) - pos) as usize])
                    .unwrap();
                file.write_all(&(new_offset as u64).to_ne_bytes()).unwrap(); //Offset will be recorded as ne_bytes ([u8;8?- due to u64])
                file.flush().unwrap();
                new_prefix = false;
            }
            prefix_i = 0;
            continue;
        } else if prefix_i == 0 {
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
        }
    }
}

/*
Function to convert token file with repeating words to a "single word"-"single occurence" file
*/
fn create_index_file(path: &str) {
    let stdin = stdin();
    let mut word = String::new();
    //Use to read Latin1
    let mut rdr = BufReader::new(
        DecodeReaderBytesBuilder::new()
            .encoding(Some(WINDOWS_1252))
            .build(
                File::open("token.txt")
                    .expect("You must have a valid token.txt file in same file as src/"),
            ),
    );
    let mut file = File::create(path).unwrap();

    for line in rdr.lines().map(|_lines| _lines.unwrap()) {
        let line = line.trim();
        let words: Vec<&str> = line.split_ascii_whitespace().collect();
        if &word != words.get(0).unwrap() {
            file.write_all(&[b'\n']).unwrap();
            file.flush().unwrap();
            word = words.get(0).unwrap().to_string();
            file.write_all(
                &WINDOWS_1252
                    .encode(&format!("{:03} {}", word, words.get(1).unwrap()))
                    .0,
            )
            .unwrap();
        } else {
            file.write_all(
                &WINDOWS_1252
                    .encode(&format!(" {}", words.get(1).unwrap()))
                    .0,
            )
            .unwrap();
        }
    }
    file.flush().unwrap();
}
