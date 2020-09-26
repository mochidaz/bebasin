use std::{
    fs::File,
    io::{self, Error},
    path::Path,
};

fn md5_digest<R: io::Read>(mut reader: R) -> Result<md5::Digest, Error> {
    let mut context = md5::Context::new();
    let mut buffer = [0; 1024];

    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        context.consume(&buffer[..count]);
    }

    Ok(context.compute())
}

pub fn md5_digest_of_file<P: AsRef<Path>>(path: &P) -> Result<md5::Digest, Error> {
    let file = File::open(path)?;
    md5_digest(file)
}

