const CORACAO: [&str; 5] = [
    ".R.R.",
    "RRRRR",
    "RRRRR",
    ".RRR.",
    "..R.."
];

const SORRISO: [&str; 5] = [
    ".....",
    ".Y.Y.",
    ".....",
    "Y...Y",
    ".YYY."
];

const COELHO: [&str; 5] = [
    "W...W",
    "W...W",
    "WWWWW",
    "WCWCW",
    "WWPWW"
];

const SUPER_MARIO: [&str; 5] = [
    ".RRR.",
    ".RRRR",
    ".WWW.",
    "WBBBW",
    ".B.B.",
];

const FLOR: [&str; 5] = [
    "..PP.",
    "..PP.",
    ".G...",
    "GGL..",
    ".LLL."
];

const FANSTASMA: [&str; 5] = [
    ".WWW.",
    "WRWRW",
    "WWWWW",
    "WWWWW",
    "W.W.W"
];

const FOGUETE: [&str; 5] = [
    "..P..",
    ".PPP.",
    ".PPP.",
    "PPPPP",
    ".O.O."
];

const ESTRELA: [&str; 5] = [
    "..L..",
    ".LYL.",
    "LYLYL",
    ".LYL.",
    "..L.."
];

const ESPADA: [&str; 5] = [
        "..B..",
        "..B..",
        "..B..",
        ".WWW.",
        "..W.."
];

const NOTA_MUSICAL: [&str; 5] = [
    "..WW.",
    "..W.W",
    "..W..",
    "WWW..",
    "WWW.."
];

const FORMIGA: [&str; 5] = [
    ".....",
    "B...B",
    ".BBB.",
    "BYBYB",
    "BBBBB"
];

pub const CORACAO_ANIMATION: [[&str; 5]; 6] = [
    [
        ".....",
        ".....",
        ".....",
        ".....",
        "....."
    ],
    [
        ".....",
        ".....",
        ".....",
        ".....",
        "..R.."
    ],
    [
        ".....",
        ".....",
        ".....",
        ".RRR.",
        "..R.."
    ],
    [
        ".....",
        ".....",
        "R...R",
        ".RRR.",
        "..R.."
    ],
    [
        ".R.R.",
        "R...R",
        "R...R",
        ".RRR.",
        "..R.."
    ],
    [
        ".R.R.",
        "RRRRR",
        "RRRRR",
        ".RRR.",
        "..R.."
    ]
];

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
    SUPER_MARIO,
];

pub const LED_MAP: [usize; 25] = [
    24, 23, 22, 21, 20,
    15, 16, 17, 18, 19,
    14, 13, 12, 11, 10,
    5,  6,  7,  8,  9,
    4,  3,  2,  1,  0
];