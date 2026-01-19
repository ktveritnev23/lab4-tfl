use std::collections::HashSet;

use std::path::Path;

use std::io::{BufRead, BufReader};

use std::fs::File;

use rand::prelude::IndexedRandom;

use rand::Rng;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Production {
    pub lhs: String,
    pub rhs: Vec<String>,
}

pub struct Grammar {
    pub productions: Vec<Production>,
    pub start_symbol: String,
    pub terminals: HashSet<String>,
    pub nonterminals: HashSet<String>,
}

// Used to generate random words
impl Grammar {
    pub fn read_cfg(filename: &str) -> Result<Self, String> {
        let path = Path::new(filename);
        let file = match File::open(&path) {
            Ok(f) => f,
            Err(e) => return Err(format!("Error opening file '{filename}': {e}")),
        };

        let reader = BufReader::new(file);
        let mut productions: Vec<Production> = Vec::new();
        let mut start_symbol = String::new();
        let mut terminals = HashSet::new();
        let mut nonterminals = HashSet::new();

        for (index, line) in reader.lines().enumerate() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("Error reading line {}: {}", index, e);
                    continue;
                }
            };

            let trim_line = line.trim();
            if trim_line.is_empty() {
                continue;
            }

            let parts: Vec<&str> = trim_line.split("->").collect();
            if parts.len() != 2 {
                eprintln!("Skipping malformed line (missing '->'): {}", trim_line);
                continue;
            }

            let lhs = parts[0].trim().to_string();
            nonterminals.insert(lhs.clone());

            let rhs_str = parts[1].trim();

            if index == 0 {
                start_symbol = lhs.clone();
            }

            let epsilon_synonyms = ["", "ε", r"\varepsilon", "epsilon"];

            let rhs: Vec<String> = if rhs_str.is_empty() || epsilon_synonyms.contains(&rhs_str) {
                Vec::new()
            } else {
                rhs_str.split_whitespace().map(|s| s.to_string()).collect()
            };

            for symbol in &rhs {
                terminals.insert(symbol.clone());
            }

            productions.push(Production { lhs, rhs });
        }

        terminals.retain(|x| !nonterminals.contains(x));

        Ok(Grammar {
            productions,
            start_symbol,
            terminals,
            nonterminals,
        })
    }

    pub fn print_cfg(&self) {
        println!("Set of non-terminal symbols:");
        for nt in &self.nonterminals {
            println!("{}", nt);
        }
        println!("Set of terminal symbols:");
        for nt in &self.terminals {
            println!("{}", nt);
        }
        println!("Start symbol: {}", &self.start_symbol);
        println!("Productions:");
        for (i, prod) in self.productions.iter().enumerate() {
            let rhs_stringify = if prod.rhs.is_empty() {
                "ε".to_string()
            } else {
                prod.rhs.join(" ")
            };
            println!("{}. {} -> {}", i, prod.lhs, rhs_stringify);
        }
    }

    pub fn generate_random_word(&self, max_depth: usize) -> Option<String> {
        let mut rng = rand::rng();

        // Internal recursive function to expand symbols
        fn expand(
            symbol: &str,
            grammar: &Grammar,
            depth: usize,
            max_depth: usize,
            rng: &mut rand::rngs::ThreadRng,
        ) -> Option<String> {
            if depth > max_depth {
                return None;
            }

            // If symbol is not a non-terminal, it is a terminal (base case)
            if !grammar.nonterminals.contains(symbol) {
                return Some(symbol.to_string());
            }

            // Find all productions for the current LHS (symbol)
            let applicable_productions: Vec<_> = grammar
                .productions
                .iter()
                .filter(|p| p.lhs == symbol)
                .collect();

            if applicable_productions.is_empty() {
                return None;
            }

            // Pick a random production
            let production = applicable_productions.choose(rng)?;

            let mut result = String::new();
            // Expand all symbols in the RHS of the chosen production
            for s in &production.rhs {
                match expand(s, grammar, depth + 1, max_depth, rng) {
                    Some(expanded_part) => result.push_str(&expanded_part),
                    None => return None, // Propagate failure up the recursion
                }
            }
            Some(result)
        }

        expand(&self.start_symbol, self, 0, max_depth, &mut rng)
    }

    /// Generates a string that is guaranteed NOT to be in the language.
    /// Strategy: 
    /// 1. Generate a valid word.
    /// 2. Apply a random mutation (Insert, Delete, or Substitute).
    /// 3. If the valid word is empty or generation fails, generate random noise.
    pub fn generate_invalid_word(&self, max_depth: usize) -> Option<String> {
        let mut rng = rand::rng();

        // Attempt to get a valid base string
        if let Some(valid_word) = self.generate_random_word(max_depth) {
            if valid_word.is_empty() {
                // If the word is empty (epsilon), we can't delete/substitute characters.
                // Just return a single random terminal character.
                if let Some(c) = self.get_random_terminal_char(&mut rng) {
                    return Some(c.to_string());
                }
            } else {
                // Corrupt the valid word
                return Some(self.mutate_string(&valid_word, &mut rng));
            }
        }

        // Fallback: If we couldn't generate a valid word (depth too low?), 
        // generate a random string of "noise" using the grammar's alphabet.
        let length = rng.random_range(1..=std::cmp::max(1, max_depth));
        Some(self.generate_random_noise(length, &mut rng))
    }

    /// Applies a single random mutation to the string.
    fn mutate_string(&self, s: &str, rng: &mut rand::rngs::ThreadRng) -> String {
        let mut chars: Vec<char> = s.chars().collect();
        let term_chars: Vec<char> = self.terminals.iter().flat_map(|t| t.chars()).collect();

        // If we have no terminals to work with, return original (shouldn't happen in valid CFG)
        if term_chars.is_empty() {
            return s.to_string();
        }

        // Pick a random operation: 0=Insert, 1=Delete, 2=Substitute
        let operation = rng.random_range(0..3);

        match operation {
            0 => {
                // Insertion: Add a random terminal at a random position
                let pos = rng.random_range(0..=chars.len());
                if let Some(&c) = term_chars.choose(rng) {
                    chars.insert(pos, c);
                }
            }
            1 => {
                // Deletion: Remove a character at a random position
                let pos = rng.random_range(0..chars.len());
                chars.remove(pos);
            }
            _ => {
                // Substitution: Replace a character with a different terminal
                let pos = rng.random_range(0..chars.len());
                let current = chars[pos];
                
                // Try to find a different character. If all terminals are the same 
                // (e.g., grammar only has 'a'), substitution doesn't change anything,
                // so we fallback to insertion.
                let different_chars: Vec<&char> = term_chars.iter().filter(|&&c| c != current).collect();

                if let Some(&&c) = different_chars.choose(rng) {
                    chars[pos] = c;
                } else {
                    // Fallback to insertion if substitution is impossible
                    let insert_pos = rng.random_range(0..=chars.len());
                    if let Some(&c) = term_chars.choose(rng) {
                        chars.insert(insert_pos, c);
                    }
                }
            }
        }

        chars.into_iter().collect()
    }

    /// Generates a random string of 'noise' from the terminal alphabet.
    fn generate_random_noise(&self, length: usize, rng: &mut rand::rngs::ThreadRng) -> String {
        let term_chars: Vec<char> = self.terminals.iter().flat_map(|t| t.chars()).collect();
        if term_chars.is_empty() {
            return String::new();
        }

        (0..length)
            .map(|_| term_chars.choose(rng).unwrap())
            .collect()
    }

    /// Helper to get a single random terminal character.
    fn get_random_terminal_char(&self, rng: &mut rand::rngs::ThreadRng) -> Option<char> {
        let term_chars: Vec<char> = self.terminals.iter().flat_map(|t| t.chars()).collect();
        term_chars.choose(rng).copied()
    }
}


