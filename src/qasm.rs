use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use regex::Regex;
use std::{
    collections::HashMap,
    io::{self, BufRead},
};

use crate::structures::{Circuit, Gate};

pub fn parse_qasm<R: BufRead>(reader: R) -> Circuit {
    let mut qubit_next_layer: HashMap<u32, u32> = HashMap::new();
    let mut layers: Vec<Vec<Gate>> = Vec::new();
    let mut offset: u32 = 0;
    let mut register_groups: HashMap<String, u32> = HashMap::new();

    // For capturing the qubit register names
    let qreg_re = Regex::new(r"^(qreg)\s+([a-zA-Z_][a-zA-Z0-9_]*)\[(\d+)\];$").unwrap();

    for line in reader.lines().flatten() {
        if let Some(caps) = qreg_re.captures(&line) {
            let name = caps[2].to_string();
            let size: u32 = caps[3].parse().unwrap();
            register_groups.insert(name, offset);
            offset += size;
            continue;
        }

        if register_groups.is_empty() {
            continue;
        }

        let keys = register_groups
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join("|");
        // Regex for capturing cx gates
        let cx_re = Regex::new(&format!(
            r"^(cx)\s+({})\[(\d+)\],\s*({})\[(\d+)\];$",
            keys, keys
        ))
        .unwrap();
        // Regex for capturing t or tdg gates
        let t_re = Regex::new(&format!(r"^(t|tdg)\s+({})\[(\d+)\];$", keys)).unwrap();

        let maybe_gate: Option<Gate> = if let Some(caps) = cx_re.captures(&line) {
            let q0 = caps[3].parse::<u32>().unwrap() + register_groups[&caps[2]];
            let q1 = caps[5].parse::<u32>().unwrap() + register_groups[&caps[4]];
            Some(Gate::cx(q0, q1))
        } else if let Some(caps) = t_re.captures(&line) {
            let q0 = caps[3].parse::<u32>().unwrap() + register_groups[&caps[2]];
            match caps.get(1).unwrap().as_str() {
                "t" => Some(Gate::t(q0)),
                "tdg" => Some(Gate::tdg(q0)),
                _ => None,
            }
        } else {
            None
        };

        if let Some(gate) = maybe_gate {
            let (q0, maybe_q1) = gate.get_qubits();
            let mut max_layer = 0;
            max_layer = max_layer.max(*qubit_next_layer.get(&q0).unwrap_or(&0));
            if let Some(q1) = maybe_q1 {
                max_layer = max_layer.max(*qubit_next_layer.get(&q1).unwrap_or(&0))
            }

            qubit_next_layer.insert(q0, max_layer + 1);
            if let Some(q1) = maybe_q1 {
                qubit_next_layer.insert(q1, max_layer + 1);
            }

            if layers.len() == max_layer as usize {
                layers.push(Vec::new());
            }
            layers[max_layer as usize].push(gate);
        }
    }

    let mut circ = Circuit::new();
    layers
        .into_iter()
        .flatten()
        .for_each(|gate| circ.add_gate(gate));
    circ
}

pub fn parallel_parse_qasm<R: BufRead>(reader: R) -> io::Result<Circuit> {
    let mut offset: u32 = 0;
    let mut register_groups: HashMap<String, u32> = HashMap::new();

    // For capturing the qubit register names
    let qreg_re = Regex::new(r"^(qreg)\s+([a-zA-Z_][a-zA-Z0-9_]*)\[(\d+)\];$").unwrap();
    let mut gate_lines: Vec<String> = Vec::new();
    for line in reader.lines().flatten() {
        if let Some(caps) = qreg_re.captures(&line) {
            let name = caps[2].to_string();
            let size: u32 = caps[3].parse().unwrap();
            register_groups.insert(name, offset);
            offset += size;
        } else {
            gate_lines.push(line);
        }
    }

    let keys = register_groups
        .keys()
        .cloned()
        .collect::<Vec<_>>()
        .join("|");
    // Regex for capturing cx gates
    let cx_re = Regex::new(&format!(
        r"^(cx)\s+({})\[(\d+)\],\s*({})\[(\d+)\];$",
        keys, keys
    ))
    .unwrap();
    // Regex for capturing t or tdg gates
    let t_re = Regex::new(&format!(r"^(t|tdg)\s+({})\[(\d+)\];$", keys)).unwrap();

    let gates: Vec<Gate> = gate_lines
        .par_iter()
        .filter_map(|line| {
            if let Some(caps) = cx_re.captures(&line) {
                let q0 = caps[3].parse::<u32>().unwrap() + register_groups[&caps[2]];
                let q1 = caps[5].parse::<u32>().unwrap() + register_groups[&caps[4]];
                Some(Gate::cx(q0, q1))
            } else if let (Some(caps)) = t_re.captures(&line) {
                let q0 = caps[3].parse::<u32>().unwrap() + register_groups[&caps[2]];
                match caps.get(1).unwrap().as_str() {
                    "t" => Some(Gate::t(q0)),
                    "tdg" => Some(Gate::tdg(q0)),
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect();
    let mut circ = Circuit::new();
    gates.into_iter().for_each(|g| circ.add_gate(g));
    Ok(circ)
}
