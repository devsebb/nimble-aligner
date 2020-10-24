use crate::filter;
use crate::reference_library;

use std::io::{Write, Error};
use std::collections::HashMap;
use std::fs::File;

use debruijn::dna_string::DnaString;
use array_tool::vec::Intersect;
use reference_library::ReferenceMetadata;

type PseudoAligner = debruijn_mapping::pseudoaligner::Pseudoaligner<debruijn::kmer::VarIntKmer<u64, debruijn::kmer::K20>>;

pub enum IntersectLevel {
  NoIntersect,
  IntersectWithFallback,
  ForceIntersect
}

pub struct AlignFilterConfig {
  pub reference_genome_size: usize,
  pub score_threshold: usize,
  pub num_mismatches: usize,
  pub discard_differing_read_pairs: bool,   // TODO
  pub discard_nonzero_mismatch: bool,
  pub discard_multiple_matches: bool,
  pub score_filter: i32,
  pub intersect_level: IntersectLevel,
  pub debug_reference: String
}

/* Takes a set of sequences and optionally, reverse sequences, a debrujin map index of the reference
 * genome, the size of the reference genome, and the threshold to match against, and performs a
 * debrujin-graph based pseduoalignment, returning a score for each readable reference in the reference
 * genome.
 * This function does some alignment-time filtration based on the provided parameters. */
pub fn score<I>(sequences: I, mut reverse_sequences: Option<I>, index: PseudoAligner, reference_metadata: &ReferenceMetadata,
  config: &AlignFilterConfig) -> Vec<(String, i32)>
  where 
    I: Iterator<Item = Result<DnaString, Error>>
  {

  // Variables to configure debugging and header tsv info
  let mut debug_str_rep = String::new();
  debug_str_rep += "sequence\tscore\n\n\n";
  debug_str_rep += "Reference: ";
  debug_str_rep += &config.debug_reference;
  debug_str_rep += "\n\n\n";

  let mut score_map: HashMap<String, (i32, bool)> = HashMap::new();

  for read in sequences {
    for (_, value) in score_map.iter_mut() {
      *value = (value.0, false);
    }

    let read = read.expect("Error -- could not parse read. Input R1 data malformed.");
    /* Generate score and equivalence class for this read by aligning the sequence against
     * the current reference. This alignment returns any scores that are greater than the match threshold. */
    let seq_score = pseudoalign(&read, &index, config.score_threshold, config.num_mismatches,
      config.discard_multiple_matches, config.discard_nonzero_mismatch);

    let mut rev_seq_score = None;
    let mut reverse_read = DnaString::new();
    // If there's a reversed sequence, do the paired-end alignment
    if let Some(itr) = &mut reverse_sequences {
      reverse_read = itr.next().expect(
        "Error -- read and reverse read files do not have matching lengths: "
      ).expect("Error -- could not parse reverse read. Input R2 data malformed.");
      rev_seq_score = Some(pseudoalign(&reverse_read, &index, config.score_threshold, config.num_mismatches,
        config.discard_multiple_matches, config.discard_nonzero_mismatch));
    }

    /* Get the score and the associated equivalence class of the forward sequence. If there's a reverse sequence and it 
     * matches, and there wasn't a match on the forward read, use that match instead */
    let match_eqv_class = match config.intersect_level {
      IntersectLevel::NoIntersect => get_best_reads(&seq_score, &rev_seq_score),
      IntersectLevel::IntersectWithFallback => get_intersecting_reads(&seq_score, &rev_seq_score, true),
      IntersectLevel::ForceIntersect => get_intersecting_reads(&seq_score, &rev_seq_score, false)
    };

    // If there was a match, update the results accordingly
    if !match_eqv_class.is_empty() {
      for idx in match_eqv_class {
        let key = &reference_metadata.columns[reference_metadata.group_on][idx as usize];

        // Write debug output for specified reference
        if key == &config.debug_reference {
          let r1_seq = read.to_string();
          let r1_score = if let Some((_, score)) = seq_score {
            score.to_string()
          } else {
            String::from("NO MATCH")
          };

          let mut r2_seq = String::from("NO REVERSE READ");
          if reverse_read.len() != 0 {
            r2_seq = reverse_read.to_string();
          }
          let r2_score = if let Some(Some((_, score))) = rev_seq_score {
            score.to_string()
          } else {
            String::from("NO MATCH")
          };

          debug_str_rep += &r1_seq;
          debug_str_rep += "\t";
          debug_str_rep += &r1_score;
          debug_str_rep += "\n";

          debug_str_rep += &r2_seq;
          debug_str_rep += "\t";
          debug_str_rep += &r2_score;
          debug_str_rep += "\n\n";
        }

        let accessor = score_map.entry(key.to_string()).or_insert((1, true));

        if accessor.1 == false {
          accessor.0 += 1;
          accessor.1 = true;
        }
      }
    }
  }

  let mut results = Vec::new();
  for (key, value) in score_map.into_iter() {
    results.push((key, value.0));
  }

  // If debug mode is on, write the debug file to disk
  if config.debug_reference != "" {
    let mut file = File::create("debug.tsv").expect("Error -- could not create debug file");
    file.write_all(debug_str_rep.as_bytes()).expect("Error -- could not write debug info to file");
    println!("Debug results written to ./debug.tsv");
  }
  results
}


// Return matches that match in both seq_score and rev_seq_score; if soft intersection is enabled, fall back to best read if one of the reads is empty
fn get_intersecting_reads(seq_score: &Option<(Vec<u32>, usize)>, rev_seq_score: &Option<Option<(Vec<u32>, usize)>>, fallback_on_intersect_fail: bool) -> Vec<u32> {
  if let (Some((eqv_class_seq, _)), Some(Some((eqv_class_rev_seq, _)))) = (&seq_score, &rev_seq_score) {
    eqv_class_seq.intersect(eqv_class_rev_seq.to_vec())
  } else {
    if fallback_on_intersect_fail {
      get_best_reads(seq_score, rev_seq_score)
    } else {
      Vec::new()
    }
  }
}


// Return matches from seq_score -- otherwise, return matches from rev_seq_score
fn get_best_reads(seq_score: &Option<(Vec<u32>, usize)>, rev_seq_score: &Option<Option<(Vec<u32>, usize)>>) -> Vec<u32> {
  if let Some((eqv_class, _)) = &seq_score {
      (*eqv_class).clone()
  } else if let Some(Some((eqv_class, _))) = &rev_seq_score {
    (*eqv_class).clone()
  } else {
    Vec::new()
  }
}




// Align the given sequence against the given reference with a score threshold
fn pseudoalign(sequence: &DnaString, reference_index: &PseudoAligner,
  match_threshold: usize, allowed_mismatches: usize, discard_multiple_matches: bool, discard_nonzero_mismatch: bool) -> Option<(Vec<u32>, usize)> {
  // Perform alignment
  match reference_index.map_read_with_mismatch(sequence, allowed_mismatches) {
    Some((equiv_class, score, mismatches)) => {
      
      // Filter nonzero mismatch
      if discard_nonzero_mismatch && mismatches != 0 {
        return None
      }

      // Filter by score and match threshold
      filter::align::filter_alignment_by_metrics(score, equiv_class, match_threshold, discard_multiple_matches)
    },
    None => None
  }
}
