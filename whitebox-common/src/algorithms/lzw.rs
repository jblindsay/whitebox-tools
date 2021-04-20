// use std::collections::HashMap;
// use std::io::{Error, ErrorKind};

// pub fn lzw_encode(data: &[u8]) -> Result<Vec<u32>, Error> {
//     // Build initial list.
//     let mut list: HashMap<Vec<u8>, u32> = (0u32..=255).map(|i| (vec![i as u8], i)).collect();

//     let mut w = Vec::new();
//     let mut compressed = Vec::new();

//     for &b in data {
//         let mut wc = w.clone();
//         wc.push(b);

//         if list.contains_key(&wc) {
//             w = wc;
//         } else {
//             // Write w to output.
//             compressed.push(list[&w]);

//             // wc is a new sequence; add it to the list.
//             list.insert(wc, list.len() as u32);
//             w.clear();
//             w.push(b);
//         }
//     }

//     // Write remaining output if necessary.
//     if !w.is_empty() {
//         compressed.push(list[&w]);
//     }

//     Ok(compressed)
// }

// pub fn lzw_decode(mut data: &[u32]) -> Result<Vec<u8>, Error> {
//     // Build the list.
//     let mut list: HashMap<u32, Vec<u8>> = (0u32..=255).map(|i| (i, vec![i as u8])).collect();

//     let mut w = list[&data[0]].clone();
//     data = &data[1..];
//     let mut decompressed = w.clone();

//     for &k in data {
//         let entry = if list.contains_key(&k) {
//             list[&k].clone()
//         } else if k == list.len() as u32 {
//             let mut entry = w.clone();
//             entry.push(w[0]);
//             entry
//         } else {
//             return Err(Error::new(
//                 ErrorKind::InvalidInput,
//                 "Error during LZW decompression.",
//             ));
//         };

//         decompressed.extend_from_slice(&entry);

//         // New sequence; add it to the list.
//         w.push(entry[0]);
//         list.insert(list.len() as u32, w);

//         w = entry;
//     }

//     Ok(decompressed)
// }
