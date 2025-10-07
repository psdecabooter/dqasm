use std::collections::HashSet;
use std::io;

pub struct Header {
    pub magic: [u8; 6],
    pub version: u16,
    pub num_qubits: u32,
    pub num_gates: u64,
}
impl Header {
    const fn dqasm_magic() -> &'static [u8; 6] {
        b"DQASM\0"
    }
    pub fn new(num_qubits: u32, num_gates: u64) -> Self {
        Self {
            magic: *Header::dqasm_magic(),
            version: 1,
            num_qubits: num_qubits,
            num_gates: num_gates,
        }
    }

    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.magic)?;
        writer.write_all(&self.version.to_le_bytes())?;
        writer.write_all(&self.num_qubits.to_le_bytes())?;
        writer.write_all(&self.num_gates.to_le_bytes())?;
        Ok(())
    }

    fn read<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let mut magic = [0u8; 6];
        reader.read_exact(&mut magic)?;
        if &magic != Header::dqasm_magic() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid file magic",
            ));
        }

        let mut buf2 = [0u8; 2];
        reader.read_exact(&mut buf2)?;
        let version = u16::from_le_bytes(buf2);

        let mut buf4 = [0u8; 4];
        reader.read_exact(&mut buf4)?;
        let num_qubits = u32::from_le_bytes(buf4);

        let mut buf8 = [0u8; 8];
        reader.read_exact(&mut buf8)?;
        let num_gates = u64::from_le_bytes(buf8);

        Ok(Header {
            magic,
            version,
            num_qubits,
            num_gates,
        })
    }
}

/// Gate types:
///
/// 0: T
///
/// 1: CX
///
/// 2: H
///
/// 3: S
#[derive(Debug)]
pub struct Gate {
    pub gate_type: u8,
    pub qubit1: u32,
    pub qubit2: u32,
}
impl Gate {
    const fn op_bits() -> usize {
        /*
        2 bits to represent:
        0: T
        1: CX
        2: H
        3: S
         */
        2
    }

    fn new(gate_type: u8, qubit1: u32, qubit2: u32) -> Self {
        Gate {
            gate_type,
            qubit1,
            qubit2,
        }
    }

    pub fn t(q: u32) -> Self {
        Gate::new(0, q, 0)
    }

    pub fn cx(q1: u32, q2: u32) -> Self {
        Gate::new(1, q1, q2)
    }

    pub fn h(q: u32) -> Self {
        Gate::new(2, q, 0)
    }

    pub fn s(q: u32) -> Self {
        Gate::new(3, q, 0)
    }

    pub fn is_double_qubit(&self) -> bool {
        self.gate_type == 1
    }

    pub fn get_qubits(&self) -> (u32, Option<u32>) {
        match self.is_double_qubit() {
            true => (self.qubit1, Some(self.qubit2)),
            false => (self.qubit1, None),
        }
    }

    fn write<W: io::Write>(&self, writer: &mut W, num_qubits: u32) -> io::Result<()> {
        let qubit_bits = (32 - (num_qubits - 1).leading_zeros()) as usize;
        let mut bit_buf = BitBuffer::new();
        bit_buf.write_bits(self.gate_type as u64, Gate::op_bits());
        bit_buf.write_bits(self.qubit1 as u64, qubit_bits);
        if self.is_double_qubit() {
            bit_buf.write_bits(self.qubit2 as u64, qubit_bits);
        }

        writer.write_all(bit_buf.bytes())?;

        Ok(())
    }

    fn read<R: io::Read>(reader: &mut R, num_qubits: u32) -> io::Result<Self> {
        let qubit_bits = (32 - (num_qubits - 1).leading_zeros()) as usize;
        let mut byte_buf = [0u8; 1];
        reader.read_exact(&mut byte_buf)?;
        let mut bit_reader = BitReader::new(Vec::from(byte_buf));
        let gate_type = bit_reader.read_bits(Gate::op_bits()) as u8;
        let is_double_qubit = gate_type == 1;

        let remaining_byte_size =
            (qubit_bits * ((is_double_qubit as usize) + 1) + Gate::op_bits() + 7) / 8 - 1;
        let mut vec_buf = vec![0u8; remaining_byte_size];
        reader.read_exact(&mut vec_buf)?;
        bit_reader.append(&mut vec_buf);

        // read qubit size
        let qubit1 = bit_reader.read_bits(qubit_bits) as u32;
        let qubit2 = match is_double_qubit {
            true => bit_reader.read_bits(qubit_bits) as u32,
            false => 0,
        };

        Ok(Gate {
            gate_type,
            qubit1,
            qubit2,
        })
    }
}

pub struct Circuit {
    pub gates: Vec<Gate>,
    pub qubits: HashSet<u32>,
}
impl Circuit {
    pub fn new() -> Self {
        Circuit {
            gates: Vec::new(),
            qubits: HashSet::new(),
        }
    }
    pub fn add_gate(&mut self, gate: Gate) {
        let (q0, maybe_q1) = gate.get_qubits();
        if !self.qubits.contains(&q0) {
            self.qubits.insert(q0);
        }
        if let Some(q1) = maybe_q1 {
            if !self.qubits.contains(&q1) {
                self.qubits.insert(q1);
            }
        }
        self.gates.push(gate);
    }

    pub fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        let header = Header::new(self.qubits.len() as u32, self.gates.len() as u64);
        header.write(writer)?;
        self.gates
            .iter()
            .try_for_each(|g| g.write(writer, header.num_qubits))?;
        Ok(())
    }

    pub fn read<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let header = Header::read(reader)?;
        let mut circuit = Circuit::new();
        for _ in 0..header.num_gates {
            let gate = Gate::read(reader, header.num_qubits)?;
            circuit.add_gate(gate);
        }

        Ok(circuit)
    }
}

struct BitBuffer {
    data: Vec<u8>,
    bit_pos: usize,
}
impl BitBuffer {
    fn new() -> Self {
        Self {
            data: Vec::new(),
            bit_pos: 0,
        }
    }

    fn write_bits(&mut self, mut value: u64, mut bits: usize) {
        while bits > 0 {
            let byte_index = self.bit_pos / 8;
            let bit_offset = self.bit_pos % 8;

            if byte_index >= self.data.len() {
                self.data.push(0u8);
            }

            let min_bits = (8 - bit_offset).min(bits);
            let selection_mask = (1u64 << min_bits) - 1;
            let selected_bits = (value & selection_mask) as u8;

            self.data[byte_index] |= selected_bits << bit_offset;

            self.bit_pos += min_bits;
            bits -= min_bits;
            value >>= min_bits;
        }
    }

    fn bytes(&self) -> &[u8] {
        &self.data
    }
}

struct BitReader {
    data: Vec<u8>,
    bit_pos: usize,
}
impl BitReader {
    fn new(data: Vec<u8>) -> Self {
        Self { data, bit_pos: 0 }
    }

    fn append(&mut self, new_data: &mut Vec<u8>) {
        self.data.append(new_data);
    }

    fn read_bits(&mut self, mut bits: usize) -> u64 {
        let mut value = 0u64;
        let mut shift = 0;

        while bits > 0 {
            if self.bit_pos >= self.data.len() * 8 {
                panic!();
            }

            let byte_index = self.bit_pos / 8;
            let bit_offset = self.bit_pos % 8;

            let min_bits = (8 - bit_offset).min(bits);
            let selection_mask = (((1u16 << min_bits) - 1) << bit_offset) as u8;
            let selected_bits = ((self.data[byte_index] & selection_mask) >> bit_offset) as u64;

            value |= selected_bits << shift;

            self.bit_pos += min_bits;
            shift += min_bits;
            bits -= min_bits;
        }

        value
    }
}
