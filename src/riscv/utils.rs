use std::u16;

pub fn combine_parameters(args: Vec<Vec<u8>>) -> Vec<u8> {
    let mut r: Vec<u8> = Vec::new();
    for e in &args {
        let l = e.len() as u16;
        let l_byte = l.to_be_bytes();
        r.append(&mut l_byte.to_vec());
        r.extend(e);
    }
    r
}

pub fn cutting_parameters(args: Vec<u8>) -> Vec<Vec<u8>> {
    let l = args.len();
    let mut i = 0;
    let mut r: Vec<Vec<u8>> = Vec::new();
    loop {
        if i + 2 > l {
            return r;
        }
        let mut n_byte = [0x00u8; 2];
        n_byte.copy_from_slice(&args[i..i + 2]);
        i += 2;

        let n = u16::from_be_bytes(n_byte) as usize;
        if i + n as usize > l {
            return r;
        }

        r.push(args[i..i + n].to_vec());
        i += n;
    }
}
