extern crate immuno_genotyper;
extern crate debruijn;
extern crate debruijn_mapping;

use std::io::Error;
use std::collections::HashMap;
use debruijn::dna_string::DnaString;

#[test]
/* 'A02' is a portion of a the macaque MHC sequence Mamu-A1*002. A02-1 and A02-2 are 1bp and 2bp changes form A02 (see lower-case bases).  
A02-LC is the same sequence as A02, just with some upper -> lower case changes to ensure that our results are case-insensitive. */
// Case with zero mismatches
fn basic_single_strand_no_mismatch() {
  let reference_names = vec!["A02-0", "A02-1", "A02-2", "A02-LC", "KIR2DL-4"];
  let reference_names = reference_names.into_iter().map(|name| String::from(name)).collect();

  let reference_sequences = vec![
    "CGCAAGTGGGAGGCGGCGGGTGAGGCGGAGCAGCACAGAACCTACCTGGAGGGCGAGTGCCTGGAGTGGCTCCGCAGATACCTGGAGAACGGGAAGGAGACGCTGCAGCGCGCGGACCCCCCCAAGACACATGTGACCCACCACCCCGTCTCTGACCAAGAGGCCACCCTGAGGTGCTGG",
    "CGCAAGTGGGAGGCGGCGGGTGAGGCGGAGCAGCACAGAACCTACCTGGAGGGCGAGTGCCTGGAGTGGCTCCGCAGATACCTGGAGAACGGGAAGGAGACGCTcCAGCGCGCGGACCCCCCCAAGACACATGTGACCCACCACCCCGTCTCTGACCAAGAGGCCACCCTGAGGTGCTGG",
    "CGCAAGTGGGAGGCGGCGGGTGAGGCGGAGCAGCACAGAACCTACCTGGAGGGCGAGTGCCTGGAGTGGCTCCGCAGATACCTGGAGAACGGCAAGGAGACGCTcCAGCGCGCGGACCCCCCCAAGACACATGTGACCCACCACCCCcTCTCTGACCAAGAGGCCACCCTGAGGTGCTGG",
    "CGCAAGTGGGAGGCGGCGGGTGAGGCGGAGCAGCACAGAACCTACCTGGAGGGCGAGTGCCTGGAGTGGCTCCGCAGATACCTGGAGAACGGCAAGGAGACGCTcCAGCGCGCGGACCCCCCCAAGACACATGTGACCCACCACCCCcTCTCTGACCAAGAGGCCACCCTGAGGTGCTGG",
    "CACTCCCCCACTGAGTGGTCGGCACCCAGCAACCCCCTGGTGATCATGGTCACAGGTCTATATGAGAAACCTTCTCTCTCAGCCCAGCCGGGCCCCACGGTTCCCACAGGAGAGAACATGACCTTGTCCTGCAGTTCCCGGCGCTCCTTTGACATGTACCATCTATCCAGGGAGGGGGAG"];
  let reference_sequences: Vec<DnaString> = reference_sequences.into_iter().map(|seq| DnaString::from_dna_string(seq)).collect();

  let sequences = vec![
    "TACCTGGAGAACGGGAAGGAGACGCTGCAGCGCGCGGACCCCCCCAAGACACATGTGACCCACCACCCCGTCTCTGACCAAGAGGCCACCCTGAGGTGCT",                  // Test-Data-1: exact match to A02-0
    "TACCTGGAGAACGGGAAGGAGACGCTcCAGCGCGCGGACCCCCCCAAGACACATGTGACCCACCACCCCGTCTCTGACCAAGAGGCCACCCTGAGGTGCT",                  // Test-Data-2: exact match to A02-1
    "TACCTGGAGAACGGGAAGGAGACGCTcCAGCGCGCGGACCCCCCCAAGACACATGTGACCCACCACCCCGTCTCTGACCAAGAGGCCACCCTGAGGTGCTatgatgatagatag",    // Test-Data-3: exact match to A02-1, except has extraneous bases at end
    "CAAGTGGGAGGCGGCGGGTGAGGCGGAGCAGCACAGAACCTACCTGGAGGGCGAGTGCCTGGAGTGGCTCCGCAGATACCTGGAGAACGGGAAGGAGACGC"                  // Test-Data-4: exact match to 5' end of A02-0 through A02-2
  ];
  let sequences: Vec<Result<DnaString, Error>> = sequences.into_iter().map(|seq| Ok(DnaString::from_dna_string(seq))).collect();

  let reference_index = debruijn_mapping::build_index::build_index::<debruijn_mapping::config::KmerType>(
    &reference_sequences.as_slice(),
    &reference_names,
    &HashMap::new(),
    1
  ).expect("Error -- could not create pseudoaligner index of the reference library");

  let library = "header\nA02-0\nA02-1\nA02-2\nA02-LC\nKIR2DL-4";
  let library = immuno_genotyper::utils::get_tsv_reader(library.as_bytes());

  // Configure aligner
  let align_config = immuno_genotyper::align::AlignFilterConfig {
    reference_genome_size: 5,
    score_threshold: 60,
    num_mismatches: 0,
    discard_differing_read_pairs: false,
    discard_nonzero_mismatch: false,
    discard_multiple_matches: false 
  };

  let results = immuno_genotyper::score::score(sequences.into_iter(), None, reference_index, library.into_records(), align_config, 0);

  let expected_results = vec![
    (String::from("A02-0"), 2.0), 
    (String::from("A02-1"), 3.0),
    (String::from("A02-2"), 1.0), 
    (String::from("A02-LC"), 2.0),
    (String::from("KIRDL-4"), 0.0)];
  assert_eq!(results, expected_results);
}