use std::result::Result;
use std::env;
use std::io;
use std::io::{Read, Write};
use std::fs;
use std::mem;
use std::path::Path;

const BUFFERSIZE: usize = 4096;

fn append_file_content(infile: &mut fs::File, outfile: &mut fs::File) -> io::Result<()> {
    let mut buffer: [u8; BUFFERSIZE] = unsafe{ mem::uninitialized() };
    loop {
        match infile.read(&mut buffer) {
            Ok(n) => {
                if n > 0 {
                    let writebuff = &buffer[0..n];
                    match outfile.write(writebuff) {
                        Ok(_) => {},
                        Err(e) => break Err(e),
                    }
                } else {
                    // No bytes read, should mean end of infile
                    break Ok(());
                }
            },
            Err(e) => break Err(e),
        };
    }
}

fn merge_files(infileprefix: &str, outfilename: &str) -> Result<u32,(String,io::Error)> {
    // Check output directory, create if required
    let outfilepath = Path::new(&outfilename);
    let outfolder = match outfilepath.parent() {
        Some(p) => p,
        None => return Err((format!("Cannot determine parent folder of output file \"{}\"", outfilename),
                    io::Error::from(io::ErrorKind::InvalidInput))),
    };
    let outfoldername = outfolder.to_str().unwrap_or("<non UTF-8 path>");
    if !outfolder.is_dir() {
        match fs::create_dir_all(outfolder) {
            Ok(_) => {},
            Err(e) => return Err((format!("Output directory \"{}\" not found and cannot be created", outfoldername),e)),
        }
    }

    // Open outfile
    let mut outfile = match fs::OpenOptions::new().write(true).create_new(true).open(outfilepath.as_os_str()) {
        Ok(f) => f,
        Err(e) => return Err((format!("Error creating and opening \"{}\" for writing", outfilename),e)),
    };

    // Loop input files and append data to outfile
    let mut filecount: u32 = 1;
    loop {
        let infilename = format!("{}.{}", infileprefix, filecount);

        // Open infile and copy contents
        match fs::File::open(&infilename) {
            Ok(mut infile) => {
                match append_file_content(&mut infile, &mut outfile) {
                    Ok(_) => {},
                    Err(e) => return Err((format!("Error copying data from \"{}\" to \"{}\"",infilename,outfilename),e)),
                }
                filecount += 1;
            },
            Err(e) => {
                if e.kind() == io::ErrorKind::NotFound {
                    break;
                } else { 
                    return Err((format!("Error opening input file \"{}\"",infilename),e));
                }
            },
        };
    }
    Ok(filecount)
}

fn main() -> io::Result<()> {
    let args: Vec<_> = env::args().collect();
    let infile: &str;
    let outfile: &str;

    // Read args
    match args.len() {
        2...3 => {
            infile = &args[1];
            outfile = if args.len() == 3 { &args[2] } else { &infile };
        }
        _ => {
            println!("Usage:");
            println!("{} <FILES> [<OUTPUT_FILE>]", args[0]);
            return Ok(());
        }
    }

    // Merge files
    match merge_files(infile, outfile) {
        Ok(_) => Ok(()),
        Err((m,e)) => {
            println!("{}", m);
            Err(e)
        },
    }
}
