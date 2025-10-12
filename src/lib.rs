use pyo3::prelude::*;
pub mod qasm_parser;
pub mod structures;

pub fn my_function() -> String {
    "Hello".to_string()
}

#[pyfunction]
fn py_my_function() -> String {
    my_function()
}

#[pymodule]
fn dqasm(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(py_my_function, m)?)?;
    Ok(())
}
