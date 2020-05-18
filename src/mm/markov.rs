use itertools::Itertools;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use c12n;
use sequence;

/// A trained Markov model.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct MM {
    pub class_name: String,
    pub pi: Vec<f64>,
    pub a: Vec<Vec<f64>>,
}

impl MM {
    pub fn save(&mut self, filename: &str) -> Result<(), Box<dyn Error>> {
        let f = File::create(filename)?;
        serde_cbor::to_writer(f, &self)?;
        Ok(())
    }

    pub fn show(&mut self) {
        let codebook_size = self.pi.len();
        println!(
            "# class_name='{}', codebook_size={}",
            self.class_name, codebook_size,
        );

        println!("pi = {}", self.pi.iter().join(", "));
        println!(" A = ");
        for row in &self.a {
            println!("     {}", row.iter().join(", "));
        }
    }

    /// log probability of generating the symbol sequence
    pub fn log_prob_sequence(&self, seq: &sequence::Sequence) -> f64 {
        let codebook_size = self.pi.len();
        let mut p = self.pi[seq.symbols[0] as usize].log10();
        for t in 0..seq.symbols.len() - 1 {
            p += self.a[seq.symbols[t] as usize][seq.symbols[t + 1] as usize].log10();
        }
        p
    }
}

pub fn load(filename: &str) -> Result<MM, Box<dyn Error>> {
    let f = File::open(filename)?;
    let br = BufReader::new(f);
    let mm = serde_cbor::from_reader(br)?;
    Ok(mm)
}

pub fn learn(seq_filenames: &Vec<PathBuf>) -> Result<MM, Box<dyn Error>> {
    // get relevant dimensions from first given sequence;
    let seq = sequence::load(seq_filenames[0].to_str().unwrap())?;

    let class_name = seq.class_name;
    let codebook_size = seq.codebook_size as usize;

    let mut counts = vec![0_usize; codebook_size];
    let mut pi = vec![0f64; codebook_size];
    let mut a = vec![vec![0f64; codebook_size]; codebook_size];

    // init:
    for i in 0..codebook_size {
        pi[i] = 1_f64;
        counts[i] = 0;
        for j in 0..codebook_size {
            a[i][j] = 1_f64;
        }
    }

    // capture counts:  (for simplicity, let this reload that 1st sequence again)
    for seq_filename in seq_filenames {
        let filename = seq_filename.to_str().unwrap();
        let seq = sequence::load(filename)?;
        println!(" {}: '{:?}'", filename, seq.class_name);

        // conformity checks:
        if codebook_size != seq.codebook_size as usize {
            return Err(format!(
                "conformity error: codebook size: {} != {}",
                codebook_size, seq.codebook_size
            )
            .into());
        }
        if class_name != seq.class_name {
            return Err(format!(
                "conformity error: class_name: {} != {}",
                class_name, seq.class_name
            )
            .into());
        }

        // count:
        pi[seq.symbols[0] as usize] += 1_f64;
        for t in 0..seq.symbols.len() - 1 {
            counts[seq.symbols[t] as usize] += 1;
            a[seq.symbols[t] as usize][seq.symbols[t + 1] as usize] += 1_f64;
        }
    }

    // normalize:
    let num_seqs = seq_filenames.len() as f64;
    for i in 0..codebook_size {
        pi[i] /= num_seqs + codebook_size as f64;
        for j in 0..codebook_size {
            a[i][j] /= counts[i] as f64 + codebook_size as f64;
        }
    }

    Ok(MM { class_name, pi, a })
}

pub fn classify(
    mm_filenames: Vec<PathBuf>,
    seq_filenames: Vec<PathBuf>,
    show_ranked: bool,
) -> Result<(), Box<dyn Error>> {
    println!("Loading MM models");
    let models: Vec<MM> = mm_filenames
        .iter()
        .map(|n| load(n.to_str().unwrap()).unwrap())
        .collect();

    let num_models = models.len();

    let mut c12n = c12n::C12nResults::new(num_models);

    println!("Classifying sequences");
    for filename in seq_filenames {
        let seq = sequence::load(filename.to_str().unwrap())?;

        let class_id_opt = &models.iter().position(|m| m.class_name == seq.class_name);
        if let Some(class_id) = *class_id_opt {
            let probs: Vec<f64> = models.iter().map(|m| m.log_prob_sequence(&seq)).collect();
            c12n.add_case(class_id, probs);
        }
    }

    println!();

    let class_names: Vec<&String> = models.iter().map(|m| &m.class_name).collect();
    c12n.report_results(class_names, "mm-classification.json".to_string());

    Ok(())
}
