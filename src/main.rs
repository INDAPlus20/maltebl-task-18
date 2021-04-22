use std::{
    fs::File,
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
    //Open or generate files
    let mut korpus = File::open("korpus").expect("Could not find korpus file to search");

    //... Index File
    let mut index_file = File::open("IndexFile.txt");
    if index_file.is_err() {
        println!("Couldn't find IndexFile.txt, generating a new one...");
        create_index_file("IndexFile.txt");
        index_file = File::open("IndexFile.txt");
    }
    let mut index_file = BufReader::new(index_file.expect("Failed to find or generate index file"));

    //... magic.txt
    let mut magic_file = File::open("magic.txt");
    if magic_file.is_err() {
        println!("Couldn't find magic.txt, generating a new one...");
        generate_magic_file("magic.txt");
        magic_file = File::open("magic.txt");
    }
    let mut magic_file = magic_file.expect("Failed to find or generate magic file");

    //Handle input and run concordance
    let stdin = stdin();
    loop {
        let mut input = String::new();
        println!("Enter word to search");
        stdin
            .read_line(&mut input)
            .expect("Did not enter a correct string");
        let result = lookup(&input.trim(), &mut korpus, &mut index_file, &mut magic_file);
        if result.is_err() {
            println!("{}", result.unwrap_err().to_string());
        }
    }
}

fn lookup(
    word_s: &str,
    korpus: &mut File,
    index_file: &mut BufReader<File>,
    magic_file: &mut File,
) -> io::Result<()> {
    //Lookup in magic file!
    let w = WINDOWS_1252.encode(&word_s).0;
    let mut word = [32; 3];
    //take prefix...
    for i in 0..w.len().min(3) {
        word[i] = w[i];
    }
    //hash it...
    let hash = hash(&word[..3]);
    let mut buff_word = [0; 8];
    //find where words with wanted prefix are in Index_File through lookup in magic.txt
    magic_file.seek(SeekFrom::Start(hash))?;
    magic_file.read_exact(&mut buff_word)?;

    //find next prefix present in Index_file to know upper boundary of search
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
    //Get all occurances of specific word by checking indicies in Index_File from offset gotten from magic.txt
    let result = check_word(word_offset, next_offset, index_file, &word_s);
    let indicies = result?;
    let mut occurences = Vec::new();
    let mut buff_sentence = [0; PREVIEW_LEN]; //30 characters long preview
                                              //Get preview passages direct from korpus
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
    //Preview a subset of all occurences and go through via user input
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
    Ok(())
}
fn check_word(
    mut word_offset: u64,
    next_offset: u64,
    i_file: &mut BufReader<File>,
    word: &str,
) -> io::Result<Vec<u64>> {
    let mut read_word = Vec::with_capacity(10);
    let mut false_line = Vec::with_capacity(50);

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
    for i in 0..=2 {
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
*/
fn generate_magic_file(path: &str) {
    let mut prefix_i = 0;
    let mut new_offset = 0;
    let mut new_prefix = false;
    let mut prefix: [u8; 3] = [0; 3];
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
                //Padd difference between end of previous prefix's 8 byte index (pos) and start of current prefix's 8 byte index with ' ' characters
                file.write_all(&vec![b' '; (hash(&prefix) - pos) as usize])
                    .unwrap();
                //Write this prefix's index (8 bytes)
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
    let mut word = String::new();
    //Use to read Latin1
    let rdr = BufReader::new(
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
