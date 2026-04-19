use crate::pokemon::PokemonName;

impl PokemonName {
    /*
    Base Stats are only required to re-calculate stats when a pokemon changes forme
    so not every pokemon will be here
    */
    pub fn base_stats(&self) -> (i16, i16, i16, i16, i16, i16) {
        match self {
            PokemonName::MINIOR => (60, 100, 60, 100, 60, 120),
            PokemonName::MINIORMETEOR => (60, 60, 100, 60, 100, 60),
            PokemonName::WISHIWASHI => (45, 20, 20, 25, 25, 40),
            PokemonName::WISHIWASHISCHOOL => (45, 140, 130, 140, 135, 30),
            PokemonName::PALAFIN => (100, 70, 72, 53, 62, 100),
            PokemonName::PALAFINHERO => (100, 160, 97, 106, 87, 100),
            PokemonName::EISCUE => (75, 80, 110, 65, 90, 50),
            PokemonName::EISCUENOICE => (75, 80, 70, 65, 50, 130),
            PokemonName::TERAPAGOSSTELLAR => (95, 95, 110, 110, 110, 85),
            PokemonName::TERAPAGOSTERASTAL => (160, 105, 110, 130, 110, 85),

            // megas
            PokemonName::VENUSAURMEGA => (80, 100, 123, 122, 120, 80),
            PokemonName::CHARIZARDMEGAX => (78, 130, 111, 130, 85, 100),
            PokemonName::CHARIZARDMEGAY => (78, 104, 78, 159, 115, 100),
            PokemonName::BLASTOISEMEGA => (79, 103, 120, 135, 115, 78),
            PokemonName::BEEDRILLMEGA => (65, 150, 40, 15, 80, 145),
            PokemonName::PIDGEOTMEGA => (83, 80, 80, 135, 80, 121),
            PokemonName::ALAKAZAMMEGA => (55, 50, 65, 175, 105, 150),
            PokemonName::SLOWBROMEGA => (95, 75, 180, 130, 80, 30),
            PokemonName::GENGARMEGA => (60, 65, 80, 170, 95, 130),
            PokemonName::KANGASKHANMEGA => (105, 125, 100, 60, 100, 100),
            PokemonName::PINSIRMEGA => (65, 155, 120, 65, 90, 105),
            PokemonName::GYARADOSMEGA => (95, 155, 109, 70, 130, 81),
            PokemonName::AERODACTYLMEGA => (80, 135, 85, 70, 95, 150),
            PokemonName::MEWTWOMEGAX => (106, 190, 100, 154, 100, 130),
            PokemonName::MEWTWOMEGAY => (106, 150, 70, 194, 120, 140),
            PokemonName::AMPHAROSMEGA => (90, 95, 105, 165, 110, 45),
            PokemonName::STEELIXMEGA => (75, 125, 230, 55, 95, 30),
            PokemonName::SCIZORMEGA => (70, 150, 140, 65, 100, 75),
            PokemonName::HERACROSSMEGA => (80, 185, 115, 40, 105, 75),
            PokemonName::HOUNDOOMMEGA => (75, 90, 90, 140, 90, 115),
            PokemonName::TYRANITARMEGA => (100, 164, 150, 95, 120, 71),
            PokemonName::SCEPTILEMEGA => (70, 110, 75, 145, 85, 145),
            PokemonName::BLAZIKENMEGA => (80, 160, 80, 130, 80, 100),
            PokemonName::SWAMPERTMEGA => (100, 150, 110, 95, 110, 70),
            PokemonName::GARDEVOIRMEGA => (68, 85, 65, 165, 135, 100),
            PokemonName::SABLEYEMEGA => (50, 85, 125, 85, 115, 20),
            PokemonName::MAWILEMEGA => (50, 105, 125, 55, 95, 50),
            PokemonName::AGGRONMEGA => (70, 140, 230, 60, 80, 50),
            PokemonName::MEDICHAMMEGA => (60, 100, 85, 80, 85, 100),
            PokemonName::MANECTRICMEGA => (70, 75, 80, 135, 80, 135),
            PokemonName::SHARPEDOMEGA => (70, 140, 70, 110, 65, 105),
            PokemonName::CAMERUPTMEGA => (70, 120, 100, 145, 105, 20),
            PokemonName::ALTARIAMEGA => (75, 110, 110, 110, 105, 80),
            PokemonName::BANETTEMEGA => (64, 165, 75, 93, 83, 75),
            PokemonName::ABSOLMEGA => (65, 150, 60, 115, 60, 115),
            PokemonName::GLALIEMEGA => (80, 120, 80, 120, 80, 100),
            PokemonName::SALAMENCEMEGA => (95, 145, 130, 120, 90, 120),
            PokemonName::METAGROSSMEGA => (80, 145, 150, 105, 110, 110),
            PokemonName::LATIASMEGA => (80, 100, 120, 140, 150, 110),
            PokemonName::LATIOSMEGA => (80, 130, 100, 160, 120, 110),
            PokemonName::RAYQUAZAMEGA => (105, 180, 100, 180, 100, 115),
            PokemonName::LOPUNNYMEGA => (65, 136, 94, 54, 96, 135),
            PokemonName::GARCHOMPMEGA => (108, 170, 115, 120, 95, 92),
            PokemonName::LUCARIOMEGA => (70, 145, 88, 140, 70, 112),
            PokemonName::ABOMASNOWMEGA => (90, 132, 105, 132, 105, 30),
            PokemonName::GALLADEMEGA => (68, 165, 95, 65, 115, 110),
            PokemonName::AUDINOMEGA => (103, 60, 126, 80, 126, 50),
            PokemonName::DIANCIEMEGA => (50, 160, 110, 160, 110, 110),

            _ => panic!("Base stats not implemented for {}", self),
        }
    }
}
