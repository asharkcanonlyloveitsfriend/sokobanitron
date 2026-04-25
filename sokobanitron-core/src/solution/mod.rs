mod validate;

pub use validate::{
    IndexedSolution, IndexedSolutionPath, PreparedPuzzle, Solution, SolutionPath, ValidationError,
    ValidationScratch, validate_indexed_solution_lines, validate_solution_lines,
};
