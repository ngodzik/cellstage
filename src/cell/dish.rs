//! One cell in a dish, its network running live: the fuller cell of
//! chapter 28, under an oxygen, a growth factor and a stress the learner
//! sets, with any of its genes removed or stuck on.
//!
//! The network is cellweave's own, compiled and stepped by the engine; what
//! the cell does with its levels is decided against the engine's own
//! thresholds. Nothing here is drawn for effect except how the dish looks,
//! which is the microscope's business.

use cellweave_core::network::Term;
use cellweave_core::{CellInputs, Concentration, NetworkSpec, SignalingNetwork, TimeStep};
use cellweave_engine::Network;
use cellweave_engine::simulation::{DEATH_THRESHOLD, QUIESCENCE_THRESHOLD};

/// Units of network time per step of the dish: the engine's own, so a step
/// here is a lattice step there.
const NETWORK_TIME_PER_STEP: u32 = 8;

/// Steps a cycling cell takes from one division to the next: the median of
/// the cycles chapter 27 draws.
pub const CYCLE_STEPS: f32 = 20.0;

/// The growth factor every cell of a simulation reads.
pub const USUAL_EGF: f64 = 0.2;

/// The fuller cell's proteins, by the names its network file gives them.
pub const GENES: [&str; 18] = [
    "ras", "erk", "hif1", "vegf", "bcl2", "p53", "caspase", "egfr", "raf", "mek", "cyclin_d", "rb",
    "e2f", "cyclin_e", "p27", "p21", "mdm2", "bax",
];

/// How a protein of the network is written for a reader.
pub fn label(name: &str) -> &str {
    match name {
        "egfr" => "EGFR",
        "ras" => "RAS",
        "raf" => "RAF",
        "mek" => "MEK",
        "erk" => "ERK",
        "hif1" => "HIF-1α",
        "vegf" => "VEGF",
        "bcl2" => "BCL2",
        "bax" => "BAX",
        "mdm2" => "MDM2",
        "rb" => "RB",
        "e2f" => "E2F",
        "cyclin_d" => "cyclin D",
        "cyclin_e" => "cyclin E",
        other => other,
    }
}

/// What has been done to a gene.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Switch {
    /// Nothing: the protein follows its terms.
    Normal,
    /// Removed: the protein is never made.
    Removed,
    /// Stuck on, as chapter 2's RAS mutant: held near full activity by
    /// nothing upstream.
    StuckOn,
}

/// What the cell is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Fate {
    /// Alive and going round its cycle, this far through it.
    Cycling(f32),
    /// Alive, its cycle stopped.
    Arrested,
    /// Taking itself apart, this far through.
    Apoptotic(f32),
    /// Dead of energy failure, this far through swelling and bursting.
    Necrotic(f32),
}

impl Fate {
    /// Whether the cell is still alive.
    pub fn alive(&self) -> bool {
        matches!(self, Fate::Cycling(_) | Fate::Arrested)
    }
}

/// The cell, and what it sits in.
pub struct Dish {
    /// The network as the file states it.
    spec: NetworkSpec,
    /// The network as the switches leave it, running.
    network: Network,
    /// What it reads: the share of the source's oxygen, the growth factor,
    /// the stress.
    pub oxygen: f64,
    pub egf: f64,
    pub stress: f64,
    /// What has been done to each gene, by node.
    switches: Vec<Switch>,
    pub fate: Fate,
    /// Steps taken since the dish was last reset.
    pub steps: u64,
    /// Divisions since then.
    pub divisions: u32,
    /// Every step's levels, oldest first, for the plot.
    pub history: Vec<Vec<f64>>,
    /// The state of the dish's own random draws, for apoptosis.
    seed: u64,
}

impl Dish {
    /// A well-fed cell with every gene as it is, carrying the fuller cell
    /// that cellweave ships.
    pub fn new() -> anyhow::Result<Self> {
        let spec = cellweave_io::shipped_network("fuller-cell")
            .map_err(|e| anyhow::anyhow!("the fuller cell: {e}"))?;
        let network = Network::from_spec(&spec, 200).map_err(|e| anyhow::anyhow!("{e}"))?;
        let switches = vec![Switch::Normal; spec.nodes.len()];
        Ok(Self {
            spec,
            network,
            oxygen: 1.0,
            egf: USUAL_EGF,
            stress: 0.0,
            switches,
            fate: Fate::Cycling(0.0),
            steps: 0,
            divisions: 0,
            history: Vec::new(),
            seed: 0x9E37_79B9_7F4A_7C15,
        })
    }

    /// The proteins, by the names the file gives them.
    pub fn names(&self) -> &[String] {
        self.network.node_names()
    }

    /// Every protein's level now.
    pub fn levels(&self) -> Vec<f64> {
        self.network.state().values.iter().map(|c| c.0).collect()
    }

    /// What has been done to a gene.
    pub fn switch(&self, node: usize) -> Switch {
        self.switches.get(node).copied().unwrap_or(Switch::Normal)
    }

    /// Remove a gene, stick it on, or put it back; the network is rebuilt
    /// from where its levels are now.
    pub fn set_switch(&mut self, node: usize, switch: Switch) {
        if self.switch(node) == switch || node >= self.switches.len() {
            return;
        }
        self.switches[node] = switch;
        let levels = self.levels();
        self.network = self.compiled(&levels);
    }

    /// The network as the switches leave it, starting from `levels`.
    fn compiled(&self, levels: &[f64]) -> Network {
        let spec = switched(&self.spec, &self.switches, levels);
        Network::from_spec(&spec, 200).unwrap_or_else(|_| self.network.clone())
    }

    /// What the cell reads.
    pub fn inputs(&self) -> CellInputs {
        inputs(self.oxygen, self.egf, self.stress)
    }

    /// One step: the network runs for a lattice step's worth of its time,
    /// then the cell is judged as the engine judges it.
    pub fn step(&mut self) {
        if !self.fate.alive() {
            self.fate = match self.fate {
                Fate::Apoptotic(p) => Fate::Apoptotic((p + 0.08).min(1.0)),
                Fate::Necrotic(p) => Fate::Necrotic((p + 0.05).min(1.0)),
                other => other,
            };
            return;
        }
        let inputs = self.inputs();
        for _ in 0..NETWORK_TIME_PER_STEP {
            self.network.step(TimeStep(1.0), &inputs);
        }
        self.steps += 1;
        self.history.push(self.levels());
        if self.history.len() > 600 {
            self.history.remove(0);
        }
        let draw = self.draw();
        self.fate = if self.network.survival() < DEATH_THRESHOLD {
            Fate::Necrotic(0.0)
        } else if self.network.apoptosis().is_some_and(|chance| draw < chance) {
            Fate::Apoptotic(0.0)
        } else if self.network.hypoxia_response() >= QUIESCENCE_THRESHOLD {
            Fate::Arrested
        } else {
            let through = match self.fate {
                Fate::Cycling(t) => t,
                _ => 0.0,
            } + 1.0 / CYCLE_STEPS;
            if through >= 1.0 {
                self.divisions += 1;
            }
            Fate::Cycling(through.fract())
        };
    }

    /// A uniform draw in [0, 1), from a generator of the dish's own.
    fn draw(&mut self) -> f64 {
        // xorshift64*: enough for a chance per step.
        let mut x = self.seed;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.seed = x;
        (x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 11) as f64 / (1u64 << 53) as f64
    }

    /// The index of a protein, by name.
    pub fn index(&self, name: &str) -> Option<usize> {
        self.names().iter().position(|n| n == name)
    }
}

/// The inputs a cell reads, crowding aside: a cell alone in a dish is not
/// squeezed.
fn inputs(oxygen: f64, egf: f64, stress: f64) -> CellInputs {
    CellInputs {
        o2: Concentration(oxygen),
        egf: Concentration(egf),
        vegf: Concentration(0.0),
        crowding: Concentration(0.0),
        stress: Concentration(stress),
        waste: Concentration(0.0),
        necrotic_factor: Concentration(0.0),
    }
}

/// `spec` with each gene as `switches` says, every node starting at
/// `levels`.
fn switched(spec: &NetworkSpec, switches: &[Switch], levels: &[f64]) -> NetworkSpec {
    let mut spec = spec.clone();
    for (i, node) in spec.nodes.iter_mut().enumerate() {
        node.initial = levels.get(i).copied().unwrap_or(node.initial);
        match switches.get(i).copied().unwrap_or(Switch::Normal) {
            Switch::Normal => {}
            Switch::Removed => {
                // Never made, and what there was is gone within a step.
                node.terms = vec![Term::Decays { rate: 1.0 }];
            }
            Switch::StuckOn => {
                // Chapter 2's mutant RAS: held on by nothing upstream.
                node.terms = vec![
                    Term::RelaxesTo {
                        value: 1.0,
                        rate: 0.5,
                    },
                    Term::Decays { rate: 0.3 },
                ];
            }
        }
    }
    spec
}

#[cfg(test)]
mod tests {
    use super::*;

    fn level(dish: &Dish, name: &str) -> f64 {
        dish.levels()[dish.index(name).unwrap()]
    }

    #[test]
    fn a_fed_cell_cycles_and_a_starved_one_stops_where_the_engine_says() {
        let mut dish = Dish::new().unwrap();
        for _ in 0..30 {
            dish.step();
        }
        assert!(matches!(dish.fate, Fate::Cycling(_)), "{:?}", dish.fate);
        assert!(dish.divisions >= 1);
        dish.oxygen = 0.2;
        for _ in 0..10 {
            dish.step();
        }
        assert_eq!(dish.fate, Fate::Arrested);
        assert!(level(&dish, "rb") >= QUIESCENCE_THRESHOLD);
    }

    #[test]
    fn a_removed_gene_falls_to_nothing_and_one_stuck_on_rises() {
        let mut dish = Dish::new().unwrap();
        let mdm2 = dish.index("mdm2").unwrap();
        let ras = dish.index("ras").unwrap();
        dish.stress = 0.5;
        dish.set_switch(mdm2, Switch::Removed);
        dish.set_switch(ras, Switch::StuckOn);
        for _ in 0..5 {
            dish.step();
            if !dish.fate.alive() {
                break;
            }
        }
        assert!(level(&dish, "mdm2") < 1e-3);
        assert!(level(&dish, "ras") > 0.5);
    }
}
