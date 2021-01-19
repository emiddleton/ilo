use std::fmt;

pub struct Rc4 {
    i: u8,
    j: u8,
    key: [u8; 16],
    pre: [u8; 16],
    state: [u8; 256],
}

impl fmt::Debug for Rc4 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let state_value = self.state[..].fmt(f);
        f.debug_struct("Rc4")
            .field("i", &self.i)
            .field("j", &self.j)
            .field("key", &self.key)
            .field("pre", &self.pre)
            .field("state", &state_value)
            .finish()
    }
}

impl Rc4 {
    pub fn new(key: &[u8]) -> Rc4 {
        assert!(key.len() == 16);

        let mut rc4 = Rc4 {
            i: 0,
            j: 0,
            key: [0; 16],
            pre: [0; 16],
            state: [0; 256],
        };
        rc4.pre[..].clone_from_slice(key);
        rc4.update_key();
        rc4
    }

    pub fn update_key(&mut self) {
        let pre_key: Vec<u8> = [&self.pre[..], &self.key[..]].concat();
        let key = md5::compute(pre_key).0;

        for (i, x) in self.state.iter_mut().enumerate() {
            *x = i as u8;
        }
        let mut j: u8 = 0;
        for i in 0..256 {
            j = j
                .wrapping_add(self.state[i])
                .wrapping_add(key[i % key.len()]);
            self.state.swap(i, j as usize);
        }
    }

    pub fn process_byte(&mut self, input: u8) -> u8 {
        input ^ self.next_byte()
    }

    pub fn process_bytes(&mut self, input: &[u8], output: &mut [u8]) {
        for (x, y) in input.iter().zip(output.iter_mut()) {
            *y = self.process_byte(*x);
        }
    }

    pub fn next_byte(&mut self) -> u8 {
        self.i = self.i.wrapping_add(1);
        self.j = self.j.wrapping_add(self.state[self.i as usize]);
        self.state.swap(self.i as usize, self.j as usize);
        self.state[(self.state[self.i as usize].wrapping_add(self.state[self.j as usize])) as usize]
    }
}

#[cfg(test)]
mod test {
    use super::Rc4 as ThisRc4;
    use crypto::{
        buffer,
        buffer::{
            BufferResult::{BufferOverflow, BufferUnderflow},
            ReadBuffer, WriteBuffer,
        },
        rc4::Rc4,
        symmetriccipher::Decryptor,
    };
    #[test]
    fn test_decryption() {
        let decrypt_key = vec![
            61, 182, 222, 9, 153, 215, 205, 204, 41, 73, 27, 188, 49, 97, 176, 184,
        ];
        let data = vec![
            0xff, 0xc0, 0x52, 0x65, 0xac, 0xf0, 0x6d, 0x2e, 0xa0, 0xdf, 0xe0, 0xc4, 0x78, 0x0d,
            0x6c, 0x63, 0x52, 0x65, 0xac, 0xf0, 0x6d, 0x2e, 0xa0, 0xdf, 0xe0, 0xc4, 0x78, 0x0d,
        ];

        let pre_key: Vec<u8> = [&decrypt_key[..], &[0; 16][..]].concat();
        let key = md5::compute(pre_key).0;
        let mut decryptor = Rc4::new(&key);

        let mut expected_1 = Vec::<u8>::new();
        let mut read_buffer = buffer::RefReadBuffer::new(&data);
        let mut buffer = [0; 1024];
        let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

        loop {
            let result = decryptor
                .decrypt(&mut read_buffer, &mut write_buffer, true)
                .unwrap();

            expected_1.extend(
                write_buffer
                    .take_read_buffer()
                    .take_remaining()
                    .iter()
                    .cloned(),
            );
            match result {
                BufferUnderflow => {
                    break;
                }
                BufferOverflow => {}
            }
        }
        let mut actual_1: Vec<u8> = vec![0; data.len()];
        let mut this_decryptor = ThisRc4::new(&decrypt_key);
        this_decryptor.process_bytes(&data, &mut actual_1);

        assert_eq!(expected_1, actual_1);

        let mut expected_2 = Vec::<u8>::new();
        let mut read_buffer = buffer::RefReadBuffer::new(&data);
        let mut buffer = [0; 1024];
        let mut write_buffer = buffer::RefWriteBuffer::new(&mut buffer);

        loop {
            let result = decryptor
                .decrypt(&mut read_buffer, &mut write_buffer, true)
                .unwrap();

            expected_2.extend(
                write_buffer
                    .take_read_buffer()
                    .take_remaining()
                    .iter()
                    .cloned(),
            );
            match result {
                BufferUnderflow => {
                    break;
                }
                BufferOverflow => {}
            }
        }
        let mut actual_2: Vec<u8> = vec![0; data.len()];
        this_decryptor.process_bytes(&data, &mut actual_2);

        assert_eq!(expected_2, actual_2);
    }
}
