/// Phase state values: 1 = positive, -1 = negative, 0 = high-impedance
pub const HIGH_IMPEDANCE: i8 = 0;

/// Each entry represents a 60° step for the three-phase trap waveform
pub const TRAP_120_MAP: [[i8; 3]; 6] = [
    [HIGH_IMPEDANCE, 1, -1],
    [-1, 1, HIGH_IMPEDANCE],
    [-1, HIGH_IMPEDANCE, 1],
    [HIGH_IMPEDANCE, -1, 1],
    [1, -1, HIGH_IMPEDANCE],
    [1, HIGH_IMPEDANCE, -1],
];

/// Each entry represents a 30° step for the three-phase trap waveform
pub const TRAP_150_MAP: [[i8; 3]; 12] = [
    [HIGH_IMPEDANCE, 1, -1],
    [-1, 1, -1],
    [-1, 1, HIGH_IMPEDANCE],
    [-1, 1, 1],
    [-1, HIGH_IMPEDANCE, 1],
    [-1, -1, 1],
    [HIGH_IMPEDANCE, -1, 1],
    [1, -1, 1],
    [1, -1, HIGH_IMPEDANCE],
    [1, -1, -1],
    [1, HIGH_IMPEDANCE, -1],
    [1, 1, -1],
];
