use dqasm::{qasm_parser::parallel_parse_qasm, structures::Circuit};
use std::{
    env, error,
    fs::File,
    io::{BufReader, BufWriter},
};

fn main() -> Result<(), Box<dyn error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        println!("Usage: cargo run <path.qasm> <out.dqasm>");
        return Ok(());
    }
    let path = &args[1];
    let out_path = &args[2];

    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let out_file = File::create(out_path)?;
    if path.ends_with(".qasm") {
        let circuit = parallel_parse_qasm(reader)?;
        let mut writer = BufWriter::new(out_file);
        circuit.write(&mut writer)?;
    } else {
        let circuit = Circuit::read(&mut reader)?;
        let mut writer = BufWriter::new(out_file);
        circuit.write(&mut writer)?;
    }

    Ok(())
}
