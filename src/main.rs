use std::{
    borrow::Cow,
    env,
    fmt::Display,
    fmt::{Formatter, Result as FmtResult},
    path::Path,
    process,
    str::FromStr,
};

use tabled::{settings::Style, Table, Tabled};

#[derive(Debug, Clone, Copy)]
enum Mode {
    AverageTime,
    SampleTime,
    SingleShotTime,
    Throughput,
}

#[derive(Debug)]
enum ParseError {
    InvalidMode,
    InvalidFloat,
    InvalidInt,
    MissingName,
    MissingCount,
}

#[derive(Debug, Tabled)]
struct BenchResult {
    name: String,
    mode: Mode,
    count: i64,
    score: f64,
    error: f64,
    units: String,
}

#[derive(Debug)]
struct BenchDiff {
    name: String,
    mode: Mode,
    old_score: f64,
    new_score: f64,
    units: String,
    diff: f64,
}

#[derive(Debug)]
struct Config {
    new_file: String,
    old_file: String,
}

impl BenchDiff {
    fn diff_str(&self) -> String {
        format!("{:+.5}%", self.diff * 100.0)
    }
}

impl Tabled for BenchDiff {
    const LENGTH: usize = 6;

    /// Fields method must return a list of cells.
    ///
    /// The cells will be placed in the same row, preserving the order.
    fn fields(&self) -> Vec<Cow<'_, str>> {
        vec![
            Cow::Owned(self.name.to_string()),
            Cow::Owned(self.mode.to_string()),
            Cow::Owned(self.old_score.to_string()),
            Cow::Owned(self.new_score.to_string()),
            Cow::Owned(self.units.to_string()),
            Cow::Owned(self.diff_str().to_string()),
        ]
    }
    /// Headers must return a list of column names.
    fn headers() -> Vec<Cow<'static, str>> {
        vec![
            Cow::Owned("name".to_string()),
            Cow::Owned("mode".to_string()),
            Cow::Owned("old count".to_string()),
            Cow::Owned("new count".to_string()),
            Cow::Owned("units".to_string()),
            Cow::Owned("diff".to_string()),
        ]
    }
}

impl Config {
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        args.next();

        let old_file = match args.next() {
            Some(arg) => arg,
            None => return Err("Didn't get a old_file path"),
        };

        let new_file = match args.next() {
            Some(arg) => arg,
            None => return Err("Didn't get a new_file path"),
        };

        Ok(Config { new_file, old_file })
    }
}

impl FromStr for Mode {
    type Err = ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "thrpt" => Ok(Self::Throughput),
            "avgt" => Ok(Self::AverageTime),
            "sample" => Ok(Self::SampleTime),
            "ss" => Ok(Self::SingleShotTime),
            _ => Err(ParseError::InvalidMode),
        }
    }
}

impl Display for Mode {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::SampleTime => write!(f, "sample"),
            Self::Throughput => write!(f, "thrpt"),
            Self::SingleShotTime => write!(f, "ss"),
            Self::AverageTime => write!(f, "avgt"),
        }
    }
}

fn parse_row(input: &str) -> Result<BenchResult, ParseError> {
    let mut parts = input.split_whitespace().fuse();

    let name = parts
        .next()
        .ok_or_else(|| ParseError::MissingName)?
        .to_string();

    let mode = parts
        .next()
        .ok_or_else(|| ParseError::InvalidMode)?
        .parse::<Mode>()?;

    let count = parts
        .next()
        .ok_or_else(|| ParseError::MissingCount)?
        .parse::<i64>()
        .map_err(|_| ParseError::InvalidInt)?;

    let score = parts
        .next()
        .ok_or_else(|| ParseError::MissingCount)?
        .parse::<f64>()
        .map_err(|_| ParseError::InvalidFloat)?;

    let error = parts
        .nth(1)
        .ok_or_else(|| ParseError::MissingCount)?
        .parse::<f64>()
        .map_err(|_| ParseError::InvalidFloat)?;

    let units = parts
        .next()
        .ok_or_else(|| ParseError::MissingCount)?
        .to_string();

    Ok(BenchResult {
        name,
        mode,
        count,
        score,
        error,
        units,
    })
}

fn parse_block(input: &str) -> (Vec<BenchResult>, Vec<ParseError>) {
    let mut errors = vec![];
    let results = input
        .split_terminator("\n")
        .enumerate()
        .filter(|&(i, _)| i > 0)
        .map(|(_, s)| parse_row(s))
        .filter_map(|r| r.map_err(|e| errors.push(e)).ok())
        .collect();
    (results, errors)
}

fn parse_file<P: AsRef<Path>>(
    path: P,
) -> Result<(Vec<BenchResult>, Vec<ParseError>), std::io::Error> {
    let file_contents = std::fs::read_to_string(path)?;
    let blocks = file_contents.split("\n\n");
    let last = blocks.last().unwrap_or_default();
    Ok(parse_block(last))
}

fn calculate_delta(new_bench_result: &BenchResult, old_bench_result: &BenchResult) -> BenchDiff {
    BenchDiff {
        name: new_bench_result.name.clone(),
        mode: new_bench_result.mode,
        new_score: new_bench_result.score,
        old_score: old_bench_result.score,
        diff: (new_bench_result.score - old_bench_result.score) / old_bench_result.score,
        units: new_bench_result.units.clone(),
    }
}

fn compare_benchmark_results(
    old_results: Vec<BenchResult>,
    new_results: Vec<BenchResult>,
) -> Vec<BenchDiff> {
    old_results
        .into_iter()
        .filter_map(|o| {
            new_results
                .iter()
                .find(|n| n.name == o.name && n.units == o.units)
                .and_then(|n| Some(calculate_delta(n, &o)))
        })
        .collect()
}

fn run(config: &Config) {
    let (new_results, new_errors) = match parse_file(&config.new_file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Problem parsing new benchmarks file: {e}");
            process::exit(1);
        }
    };

    let (old_results, old_errors) = match parse_file(&config.old_file) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Problem parsing old benchmarks file: {e}");
            process::exit(1);
        }
    };

    if old_errors.len() > 0 || new_errors.len() > 0 {
        println!("There were come errors found while parsing the benchmark results, ignoring those rows and continuing");
    }

    let result = compare_benchmark_results(old_results, new_results);

    let mut table = Table::new(result);
    table.with(Style::blank());

    println!("{}", table);
}

fn main() {
    let config = Config::build(env::args()).unwrap_or_else(|e| {
        eprintln!("Problem parsing arguments: {e}");
        process::exit(1);
    });
    run(&config);
}
