# jmhcmp

`jmhcmp` is a command line utility to compare the outputs of multiple runs of JMH benchmarks.

## Installing

To install `jmhcmp`, you need to:

1. Clone this repository
```bash
git clone https://github.com/Cali0707/jmhcmp/
```
2. Install the binary with cargo:
```bash
cargo install --path ./jmhcmp
```

## Usage

Using `jmhcmp` is really easy, you just need the results of two benchmark runs in two files. If those are called `old.txt` and `new.txt`,
then to use the tool you only need to run:
```bash
jmhcmp old.txt new.txt
```
