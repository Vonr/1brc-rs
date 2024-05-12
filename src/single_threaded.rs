use std::{
    collections::HashMap,
    fs::File,
    hash::BuildHasherDefault,
    io::{self, BufWriter, Write},
    num::NonZeroI32,
};

mod data;
mod itoa;

use data::Data;

fn main() {
    unsafe {
        let mmap = memmap2::Mmap::map(&File::open("measurements.txt").unwrap()).unwrap();
        let mut map = HashMap::<&str, Data, _>::with_capacity_and_hasher(
            10_000,
            BuildHasherDefault::<rustc_hash::FxHasher>::default(),
        );

        for line in mmap[..mmap.len() - 1].split(|&b| b == b'\n') {
            let at = line.iter().position(|&b| b == b';').unwrap_unchecked();
            let (station, [_, temp_bytes @ ..]) = line.split_at(at) else {
                std::hint::unreachable_unchecked();
            };

            let mut int = &temp_bytes[..temp_bytes.len() - 2];
            let dec = temp_bytes[temp_bytes.len() - 1];

            let temp = {
                let neg = *int.get_unchecked(0) == b'-';
                if neg {
                    int = int.get_unchecked(1..);
                };

                let temp = match int.len() {
                    1 => int.get_unchecked(0).wrapping_sub(b'0') as i16,
                    2 => {
                        int.get_unchecked(0).wrapping_sub(b'0') as i16 * 10
                            + int.get_unchecked(1).wrapping_sub(b'0') as i16
                    }
                    _ => std::hint::unreachable_unchecked(),
                } * 10
                    + dec.wrapping_sub(b'0') as i16;

                if neg {
                    -temp
                } else {
                    temp
                }
            };

            map.entry(std::str::from_utf8_unchecked(station))
                .and_modify(|e| {
                    *e = Data {
                        total: e.total.wrapping_add(temp as i32),
                        count: NonZeroI32::new_unchecked(e.count.get() + 1),
                        min: e.min.min(temp),
                        max: e.max.max(temp),
                    };
                })
                .or_insert(Data {
                    total: temp as i32,
                    count: NonZeroI32::new_unchecked(1),
                    min: temp,
                    max: temp,
                });
        }

        let mut v = map.into_iter().collect::<Box<_>>();
        glidesort::sort(&mut v);

        let stdout = io::stdout().lock();
        let mut writer = BufWriter::new(stdout);

        for (
            station,
            Data {
                total,
                count,
                min,
                max,
            },
        ) in v.into_iter()
        {
            writer.write_all(station.as_bytes()).unwrap_unchecked();
            writer
                .write_all(itoa::format(*min).as_bytes())
                .unwrap_unchecked();
            writer
                .write_all(
                    itoa::format((*total as f64 / count.get() as f64).round() as i16).as_bytes(),
                )
                .unwrap_unchecked();
            writer
                .write_all(itoa::format(*max).as_bytes())
                .unwrap_unchecked();
            writer.write_all(b"\n").unwrap_unchecked();
        }

        std::mem::forget(v);
    }
}
