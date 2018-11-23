use std::result::Result;
use std::env;
use std::io;
use std::io::{Read, Write};
use std::fs;
use std::mem;
use std::path::Path;

const BUFFERSIZE: usize = 4096;

fn parse_size(sizestr: &str) -> Option<u64> {
    let mut sizestring = sizestr.to_ascii_lowercase();
    let mut multiplier = 1;

    match sizestring.pop() {
        Some('b') => {
            match sizestring.pop() {
                Some('k') => multiplier = 1024,
                Some('m') => multiplier = 1024*1024,
                Some('g') => multiplier = 1024*1024*1024,
                Some(c) => sizestring.push(c),
                None => return None,
            }
        }
        Some(c) => sizestring.push(c),
        None => return None,
    }

    match sizestring.parse::<u64>() {
        Ok(number) => Some(number * multiplier),
        Err(_) => None,
    }
}

fn copy_file_part(infilename: &str, outfilename: &str, infile: &mut fs::File, 
        outfile: &mut fs::File, bytes_to_copy: u64) -> Result<u64,(String,io::Error)> {
    let mut buffer: [u8; BUFFERSIZE] = unsafe{ mem::uninitialized() };
    let mut written: u64 = 0;
    loop {
        let remain = bytes_to_copy - written;
        let to_read = if remain < BUFFERSIZE as u64 { remain as usize } else { BUFFERSIZE };
        let readbuff = &mut buffer[0..to_read];
        match infile.read(readbuff) {
            Ok(n) => {
                if n > 0 {
                    let writebuff = &readbuff[0..n];
                    match outfile.write(writebuff) {
                        Ok(s) => written += s as u64,
                        Err(e) => { return Err((format!("Error writing to \"{}\"", outfilename),e)) },
                    }
                } else {
                    break Ok(written); // No bytes read, should mean end of infile
                }
            },
            Err(e) => return Err((format!("Error reading from \"{}\"", infilename),e)),
        };
        if written >= bytes_to_copy { break Ok(bytes_to_copy); } // bytes_to_copy bytes written, infile EOL not reached
    }
}

fn split_file(infilename: &str, outfolder: &str, partsize: u64) -> Result<(),(String,io::Error)> {    
    // Check output directory, create if required
    let outpath = Path::new(&outfolder);
    if !outpath.is_dir() {
        match fs::create_dir_all(outpath) {
            Ok(_) => {},
            Err(e) => return Err((format!("Output directory \"{}\" not found and cannot be created", outfolder),e)),
        }
    }

    // Open infile
    let mut infile = match fs::File::open(infilename) {
        Ok(f) => f,
        Err(e) => return Err((format!("Error opening \"{}\" for reading", infilename),e)),
    };

    // Copy infile data to outfiles
    let mut filecount: u32 = 1;
    loop {
        // Open next outfile, which must not exist
        let outfilepath = outpath.join(format!("{}.{}", infilename, filecount));
        let outfilename = outfilepath.to_str().unwrap_or("<non UTF-8 path>");
        let mut outfile = match fs::OpenOptions::new().write(true).create_new(true).open(outfilepath.as_os_str()) {
            Ok(f) => f,
            Err(e) => return Err((format!("Error creating and opening \"{}\" for writing", outfilename),e)),
        };
        // Write to outfile
        match copy_file_part(infilename, outfilename, &mut infile, &mut outfile, partsize) {
            Ok(written) => {
                // Less than partsize bytes read, should mean end of infile
                if written < partsize {
                    if written == 0 {
                        drop(outfile);
                        fs::remove_file(&outfilepath).unwrap_or_default();
                    }
                    break;
                }
            },
            Err((m,e)) => return Err((m,e)),
        }
        filecount += 1;
    }
    Ok(())
}

fn main() -> io::Result<()> {
    const DEFAULT_OUTDIR: &str = ".";
    let args: Vec<_> = env::args().collect();
    let infile: &str;
    let chunksize: &str;
    let outdir: &str;

    // Read args
    match args.len() {
        3...4 => {
            infile = &args[1];
            chunksize = &args[2];
            outdir = if args.len() == 4 { &args[3] } else { DEFAULT_OUTDIR };
        }
        _ => {
            println!("Usage:");
            println!("{} <FILE> <PART_SIZE> [<OUTPUT_DIR>]", args[0]);
            return Ok(());
        }
    }
    
    // Parse <PART_SIZE> into integer and split <FILE>
    match parse_size(chunksize) {
        Some(size) => match split_file(infile, outdir, size) {
            Ok(_) => Ok(()),
            Err((m,e)) => {
                println!("{}", m);
                Err(e)
            },
        }
        None => {
            println!("Cannot parse \"{}\" as a size", chunksize);
            Ok(())
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_parse_size() {
        let values = [
            ("0", Some(0)),
            ("1", Some(1)),
            ("42", Some(42)),
            ("789B", Some(789)),
            ("789b", Some(789)),
            ("12KB", Some(12*1024)),
            ("12Kb", Some(12*1024)),
            ("12kB", Some(12*1024)),
            ("12kb", Some(12*1024)),
            ("144MB", Some(144*1024*1024)),
            ("144Mb", Some(144*1024*1024)),
            ("144mB", Some(144*1024*1024)),
            ("144mb", Some(144*1024*1024)),
            ("4GB", Some(4*1024*1024*1024)),
            ("4Gb", Some(4*1024*1024*1024)),
            ("4gB", Some(4*1024*1024*1024)),
            ("4gb", Some(4*1024*1024*1024)),
            ("-1", None),
            ("-51", None),
            ("4a", None),
            ("42bb", None),
            ("789AB", None),
        ];

        for (_i, val) in values.iter().enumerate() {
            assert_eq!(parse_size(val.0), val.1);
        }
    }
}
