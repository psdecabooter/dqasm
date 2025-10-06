use std::collections::HashSet;
use std::io;

pub struct Header {
    pub magic: [u8; 4],
    pub version: u16,
    pub num_qubits: u32,
    pub num_gates: u64,
}
impl Header {
    pub fn new(num_qubits: u32, num_gates: u64) -> Self {
        Self {
            magic: *b"BQM\0",
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
        let mut magic = [0u8; 4];
        reader.read_exact(&mut magic)?;
        if &magic != b"BQM\0" {
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
/// 0: t
///
/// 1: tdg
///
/// 2: cx
pub struct Gate {
    pub gate_type: u8,
    pub qubit1: u32,
    pub qubit2: u32,
}
impl Gate {
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

    pub fn tdg(q: u32) -> Self {
        Gate::new(1, q, 0)
    }

    pub fn cx(q1: u32, q2: u32) -> Self {
        Gate::new(2, q1, q2)
    }

    pub fn is_double_qubit(&self) -> bool {
        matches!(self.gate_type, 2)
    }

    pub fn get_qubits(&self) -> (u32, Option<u32>) {
        match self.is_double_qubit() {
            true => (self.qubit1, None),
            false => (self.qubit1, Some(self.qubit2)),
        }
    }

    fn write<W: io::Write>(&self, writer: &mut W) -> io::Result<()> {
        writer.write_all(&self.gate_type.to_le_bytes())?;
        writer.write_all(&self.qubit1.to_le_bytes())?;
        writer.write_all(&self.qubit2.to_le_bytes())?;

        Ok(())
    }

    fn read<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf1 = [0u8; 1];
        reader.read_exact(&mut buf1)?;
        let gate_type = u8::from_le_bytes(buf1);

        let mut buf4 = [0u8; 4];
        reader.read_exact(&mut buf4)?;
        let qubit1 = u32::from_le_bytes(buf4);
        reader.read_exact(&mut buf4)?;
        let qubit2 = u32::from_le_bytes(buf4);

        Ok(Self {
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
        self.gates.iter().try_for_each(|g| g.write(writer))?;
        Ok(())
    }

    pub fn read<R: io::Read>(reader: &mut R) -> io::Result<Self> {
        let header = Header::read(reader)?;
        let mut circuit = Circuit::new();
        for _ in 0..header.num_qubits {
            let gate = Gate::read(reader)?;
            circuit.add_gate(gate);
        }

        Ok(circuit)
    }
}
