use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct GenePair<A: Copy> {
    pub maternal: A,
    pub paternal: A,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum EyeAllele {
    Brown,
    Blue,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum HandAllele {
    Right,
    Left,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum DiseaseAllele {
    Healthy,
    Risk,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub enum SexAllele {
    X,
    Y,
}

#[cfg(test)]
mod tests {
    //! Gene pairs retain distinct maternal and paternal contributions at every locus.

    use super::*;

    #[test]
    fn a_gene_pair_preserves_both_parental_alleles() {
        let pair = GenePair {
            maternal: EyeAllele::Brown,
            paternal: EyeAllele::Blue,
        };
        assert_eq!(pair.maternal, EyeAllele::Brown);
        assert_eq!(pair.paternal, EyeAllele::Blue);
    }
}
