use std::io;
use csv::Reader;

pub fn get_tsv_reader<R: io::Read>(reader: R) -> Reader<R>{
  csv::ReaderBuilder::new()
    .delimiter(b'\t')
    .from_reader(reader)
}