use std::io::{self, Write};

use std::iter;

use std::time::Instant;

mod grammar;

use grammar::Grammar;

use std::fs::File;

fn main() {
    // Grammar used to generate random strings
    let grammar_string = "files/lang.txt";

    let g = match Grammar::read_cfg(&grammar_string) {
        Ok(cfg) => cfg,
        Err(msg) => panic!("Failed to read file: {}", msg),
    };
    
    // Decrease depth of productions if test takes long time
    check_approximation(&g, 20000, 200, "test_valid.csv", "test_invalid.csv");

    // Interactive mode
    println!("\n{}", "=".repeat(40));
    println!("Interactive mode (enter 'quit' to exit):");
    
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("\nEnter string to test: ");
        stdout.flush().unwrap();

        let mut user_input = String::new();
        stdin.read_line(&mut user_input).expect("Failed to read line");
        let user_input = user_input.trim();

        if user_input.eq_ignore_ascii_case("quit") {
            break;
        }

        let result = is_valid_string(user_input);
        println!("'{}' is {} in the language", user_input, if result { "VALID" } else { "INVALID" });
    }
}

/// Main function to check if a string is in the language L = L_1 or L_2 or L_3
fn parse_string(s: &str) -> bool {
    let s = s.trim();

    // Try L_1 first (simplest: ba(abb)+aaa)
    if check_l1(s) {
        return true;
    }

    // Try L_2 (with T = (bba)+bb, S can be ε)
    if check_l2_complete(s) {
        return true;
    }

    // Try L_3 (with T = bb|(bba)^2(bba)+bb, S cannot be ε)
    if check_l3_complete(s) {
        return true;
    }

    false
}

fn parse_string_optimized(s: &str) -> bool {
    let s = s.trim();

    return check_l23_optimized(s);
}

/// Check if string matches L_1: ba(abb)+aaa
fn check_l1(s: &str) -> bool {
    // Must start with 'ba' and end with 'aaa', length >= 8
    if !s.starts_with("ba") || !s.ends_with("aaa") || s.len() < 8 {
        return false;
    }

    // Remove prefix 'ba' and suffix 'aaa'
    let middle = &s[2..s.len() - 3];

    let pos = 0;
    if let Some(slice) = middle.get(pos..pos+3) {
        let mut current_pos = pos + 3;
        if slice == "abb" {
            // Continue with (a bb)* pattern
            while current_pos < middle.len() {
                if let Some(slice_a) = s.get(current_pos..current_pos + 3) {
                    if slice_a == "abb" {
                        current_pos += 3;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
        }
        return false;
    }
    middle.len() >= 3
}

// Determines whether to check L_2 or L_3, reducing redundancy
fn check_l23_optimized(s: &str) -> bool {
    if s.is_empty() {
        return  true;
    }

    if s[0..1] == *"b" && s.len() >= 8 {
        if let Some(prefix_slice) = s.get(0..5) {
            if prefix_slice == "baabb" {
                match s.get(5..7) {
                    Some("aa") => return check_l3_complete(s), // minimal T is only acceptable in L_3
                    Some("ab") => return check_l2_complete(s),
                    _ => return false,
                }
            }
        }
        return false;
    }

    if s[0..1] == *"a" && s.len() >= 4 {
        if s.len() >= 9 && let Some(prefix_slice) = s.get(0..7) {
            if prefix_slice == "abbaabb" {
                match s.get(7..9) {
                    Some("aa") => return check_l3_complete(s), // minimal T is only acceptable in L_3
                    Some("ab") => {
                        return check_l2_complete(s)||check_l3_complete(s);
                        // it still can be L_3, but probability of L_2 vs L_3 becomes higher
                    },
                    _ => return false,
                }           
            }
        }
        if let Some(prefix_slice) = s.get(0..2) {
            if prefix_slice == "ab" {
                return check_l2_complete(s)||check_l3_complete(s);
            }
        }
        return false;
    } 

    false
}

fn check_l2_complete(s: &str) -> bool {
    if s.is_empty() {
        return true; // ε production
    }

    // Try to parse as L_2 S
    if let Some(end_pos) = parse_s_l2(s, 0) {
        return end_pos == s.len();
    }
    false
}

fn check_l3_complete(s: &str) -> bool {
    if s.is_empty() {
        return false; // L_3 doesn't have ε production
    }

    // Try to parse as L_3 S
    if let Some(end_pos) = parse_s_l3(s, 0) {
        return end_pos == s.len();
    }
    false
}

/// Parse S in L_2 from position pos.
/// Returns Some(new_position) on success, None on failure.
fn parse_s_l2(s: &str, pos: usize) -> Option<usize> {
    // Try S -> baaTaaa
    if let Some(slice) = s.get(pos..pos + 3) {
        if slice == "baa" {
            if let Some(p_t) = parse_t_l2(s, pos + 3) {
                if let Some(slice_end) = s.get(p_t..p_t + 3) {
                    if slice_end == "aaa" {
                        return Some(p_t + 3);
                    }
                }
            }
        }
    }

    // Try S -> abSbbS
    if let Some(slice) = s.get(pos..pos + 2) {
        if slice == "ab" {
            // Parse first S (can be ε in L_2)
            if let Some(p1) = parse_s_l2(s, pos + 2) {
                if let Some(slice_sep) = s.get(p1..p1 + 2) {
                    if slice_sep == "bb" {
                        // Parse second S (can also be ε in L_2)
                        if let Some(p2) = parse_s_l2(s, p1 + 2) {
                            return Some(p2);
                        }
                    }
                }
            }
        }
    }

    // ε production - succeeds without consuming any input
    Some(pos)
}

/// Parse T in L_2: T -> bbaT'
fn parse_t_l2(s: &str, pos: usize) -> Option<usize> {
    if let Some(slice) = s.get(pos..pos + 3) {
        if slice == "bba" {
            return parse_t_prime(s, pos + 3);
        }
    }
    None
}

/// Parse S in L_3 from position pos.
fn parse_s_l3(s: &str, pos: usize) -> Option<usize> {
    // Try S -> baaTaaa
    if let Some(slice) = s.get(pos..pos + 3) {
        if slice == "baa" {
            if let Some(p_t) = parse_t_l3(s, pos + 3) {
                if let Some(slice_end) = s.get(p_t..p_t + 3) {
                    if slice_end == "aaa" {
                        return Some(p_t + 3);
                    }
                }
            }
        }
    }

    // Try S -> abSbbS
    if let Some(slice) = s.get(pos..pos + 2) {
        if slice == "ab" {
            // Parse first S (must be non-empty in L_3)
            if let Some(p1) = parse_s_l3(s, pos + 2) {
                // In L_3, S cannot be ε, so p1 must be > pos + 2
                if p1 > pos + 2 {
                    if let Some(slice_sep) = s.get(p1..p1 + 2) {
                        if slice_sep == "bb" {
                            // Parse second S (must be non-empty in L_3)
                            if let Some(p2) = parse_s_l3(s, p1 + 2) {
                                if p2 > p1 + 2 {
                                    return Some(p2);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // No ε production in L_3
    None
}

/// Parse T in L_3
fn parse_t_l3(s: &str, pos: usize) -> Option<usize> {
    // Option 1: T -> bb
    if let Some(slice) = s.get(pos..pos + 2) {
        if slice == "bb" {
            // Check if we have just "bb" (no following 'abbabb' pattern)
            if pos + 2 == s.len() {
                return Some(pos + 2);
            }

            // Check if the next 8 characters are not "abbabb" as it means dead production for the rest
            let has_longer_match = s.get(pos..pos + 8).map_or(false, |sub| sub == "bbabbabb")
                && s.len() >= pos + 10;
            
            if !has_longer_match {
                return Some(pos + 2);
            }
        }
    }

    // Option 2: T -> bb(abb)^2 a T'
    // Check for 'bbabbabb' followed by 'a'
    if let Some(slice1) = s.get(pos..pos + 8) {
        if slice1 == "bbabbabb" {
            if let Some(slice2) = s.get(pos + 8..pos + 9) {
                if slice2 == "a" {
                    return parse_t_prime(s, pos + 9);
                }
            }
        }
    }

    None
}

/// Parse T': bb | T'aT'
/// The core of regular part T
fn parse_t_prime(s: &str, pos: usize) -> Option<usize> {
    // Must start with 'bb'
    if let Some(slice) = s.get(pos..pos + 2) {
        if slice == "bb" {
            let mut current_pos = pos + 2;

            // Continue with (a bb)* pattern
            while current_pos < s.len() {
                // Check for 'a' followed by 'bb'
                if let Some(slice_a) = s.get(current_pos..current_pos + 3) {
                    // slice_a checks 3 chars: 'a', 'b', 'b'
                    if slice_a == "abb" {
                        current_pos += 3;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            return Some(current_pos);
        }
    }
    None
}

fn is_valid_string(s: &str) -> bool {
    parse_string(s)
}

fn is_valid_optimized(s: &str) -> bool {
    parse_string_optimized(s) 
}

use std::collections::{BTreeMap, BTreeSet};

pub fn check_approximation(
    cfg: &Grammar,
    num_tests: usize,
    max_depth: usize,
    csv_valid: &str,
    csv_invalid: &str,
) {
    println!("Generating {} valid and {} invalid words...", num_tests, num_tests);

    let valid_words: Vec<String> = iter::repeat_with(|| cfg.generate_random_word(max_depth))
        .filter_map(|x| x)
        .take(num_tests)
        .collect();

    let invalid_words: Vec<String> = iter::repeat_with(|| cfg.generate_invalid_word(max_depth))
        .filter_map(|x| x)
        .take(num_tests)
        .collect();

    println!("Generation complete.\n");

    // Data collection
    let mut valid_data: BTreeMap<usize, Vec<u128>> = BTreeMap::new();
    let mut invalid_data: BTreeMap<usize, Vec<u128>> = BTreeMap::new();

    let mut valid_data_opt: BTreeMap<usize, Vec<u128>> = BTreeMap::new();
    let mut invalid_data_opt: BTreeMap<usize, Vec<u128>> = BTreeMap::new();

    let mut true_positive = 0;
    let mut false_negative = 0;
    let mut true_negative = 0;
    let mut false_positive = 0;
    
    // Test Valid Words
    for word in &valid_words {
        let start = Instant::now();
        let is_accepted = is_valid_string(word);
        let duration = start.elapsed();
        let len = word.len();

        let new_start = Instant::now();
        let another_accepted = is_valid_optimized(word);
        let new_duration = new_start.elapsed();

        // Record time for plotting
        valid_data.entry(len).or_default().push(duration.as_nanos());
        valid_data_opt.entry(len).or_default().push(new_duration.as_nanos());


        if is_accepted && another_accepted { true_positive += 1; } 
        else { false_negative += 1; }
    }

    // Test Invalid Words
    for word in &invalid_words {
        let start = Instant::now();
        let is_accepted = is_valid_string(word);
        let duration = start.elapsed();
        let len = word.len();

        let new_start = Instant::now();
        let another_accepted = is_valid_optimized(word);
        let new_duration = new_start.elapsed();

        // Record time for plotting
        invalid_data.entry(len).or_default().push(duration.as_nanos());
        invalid_data_opt.entry(len).or_default().push(new_duration.as_nanos());
        // account for false positive, as rejected string generation can accidently give positive results (about 0-2 in 10000)
        if !is_accepted && !another_accepted { true_negative += 1; } 
        else if is_accepted && another_accepted {
            false_positive += 1;
        } else {
            false_negative += 1;
        }

    }

    println!("Test Summary:");
    println!("Expected accepted strings:  {}", num_tests + false_positive);
    println!("Expected rejected strings: {}", num_tests - false_positive);
    println!("Passed tests:  {}", true_positive + true_negative+ false_positive);
    println!("Failed tests: {}", false_negative);

    println!("\nGenerating CSV datasets....");

    match File::create(csv_valid) {
        Ok(mut file) => {
            // Write the CSV Header
            if let Err(e) = writeln!(file, "Length,Valid_Avg_Time_ns,Valid_Opt_Time_ns") {
                eprintln!("Failed to write header to file: {}", e);
                return;
            }

            let all_lengths: BTreeSet<_> = valid_data.keys().collect();

            for &len in all_lengths {
                let v_avg = calculate_average(valid_data.get(&len));
                let v_avg_opt = calculate_average(valid_data_opt.get(&len));
                if let Err(e) = writeln!(file,"{},{},{}", len, v_avg, v_avg_opt) {
                    eprintln!("Failed to write row to file: {}", e);
                    return;
                }
            }
            println!("Successfully wrote plot data to '{}'.", csv_valid);
        }
        Err(e) => {
            eprintln!("Failed to create file '{}': {}", csv_valid, e);
        }
    }

    match File::create(csv_invalid) {
        Ok(mut file) => {
            // Write the CSV Header
            if let Err(e) = writeln!(file, "Length,Invalid_Avg_Time_ns,Invalid_Opt_Time_ns") {
                eprintln!("Failed to write header to file: {}", e);
                return;
            }

            let all_lengths: BTreeSet<_> = invalid_data.keys().collect();

            for &len in all_lengths {
                let i_avg = calculate_average(invalid_data.get(&len));
                let i_avg_opt = calculate_average(invalid_data_opt.get(&len));
                if let Err(e) = writeln!(file,"{},{},{}", len, i_avg, i_avg_opt) {
                    eprintln!("Failed to write row to file: {}", e);
                    return;
                }
            }
            println!("Successfully wrote plot data to '{}'.", csv_invalid);
        }
        Err(e) => {
            eprintln!("Failed to create file '{}': {}", csv_invalid, e);
        }
    }
}

/// Helper to calculate average of a vector of times.
fn calculate_average(times: Option<&Vec<u128>>) -> f64 {
    match times {
        Some(vec) if !vec.is_empty() => {
            let sum: u128 = vec.iter().sum();
            sum as f64 / vec.len() as f64
        }
        _ => 0.0
    }
}

