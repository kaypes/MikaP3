pub mod coracao;
pub mod coelho;
pub mod espada;
pub mod estrela;
pub mod fantasma;
pub mod flor;
pub mod foguete;
pub mod formiga;
pub mod nota_musical;
pub mod sorriso;
pub mod super_mario;

use coracao::CORACAO;
use coelho::COELHO;
use espada::ESPADA;
use estrela::ESTRELA;
use fantasma::FANSTASMA;
use flor::FLOR;
use foguete::FOGUETE;
use formiga::FORMIGA;
use nota_musical::NOTA_MUSICAL;
use sorriso::SORRISO;
use super_mario::SUPER_MARIO;

pub const ARTS: [[&str; 5]; 11] = [
    CORACAO,
    COELHO,
    ESPADA,
    ESTRELA,
    FANSTASMA,
    FLOR,
    FOGUETE,
    FORMIGA,
    NOTA_MUSICAL,
    SORRISO,
    SUPER_MARIO
];