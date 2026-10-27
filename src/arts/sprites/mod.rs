pub mod coelho;
pub mod coracao;
pub mod espada;
pub mod estrela;
pub mod fantasma;
pub mod flor;
pub mod foguete;
pub mod formiga;
pub mod nota_musical;
pub mod sorriso;
pub mod super_mario;

use coelho::COELHO;
use coracao::CORACAO;
use espada::ESPADA;
use estrela::ESTRELA;
use fantasma::FANSTASMA;
use flor::FLOR;
use foguete::FOGUETE;
use formiga::FORMIGA;
use nota_musical::NOTA_MUSICAL;
use sorriso::SORRISO;
use super_mario::SUPER_MARIO;

pub type Sprite = [&'static str; 5];

pub struct Art {
    pub name: &'static str,
    pub sprite: Sprite,
}

pub static ARTS: &[Art] = &[
    Art {
        name: "CORAÇÃO",
        sprite: CORACAO,
    },
    Art {
        name: "COELHO",
        sprite: COELHO,
    },
    Art {
        name: "ESPADA",
        sprite: ESPADA,
    },
    Art {
        name: "ESTRELA",
        sprite: ESTRELA,
    },
    Art {
        name: "FANSTASMA",
        sprite: FANSTASMA,
    },
    Art {
        name: "FLOR",
        sprite: FLOR,
    },
    Art {
        name: "FOGUETE",
        sprite: FOGUETE,
    },
    Art {
        name: "FORMIGA",
        sprite: FORMIGA,
    },
    Art {
        name: "NOTA MUSICAL",
        sprite: NOTA_MUSICAL,
    },
    Art {
        name: "SORRISO",
        sprite: SORRISO,
    },
    Art {
        name: "SUPER MARIO",
        sprite: SUPER_MARIO,
    },
];
