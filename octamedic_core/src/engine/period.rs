use crate::data::instrument::{MAX_FINETUNE, MIN_FINETUNE};

// v4 period tables, C-1 to B-3, finetune -8..7. same numbers as ProTracker
const PERIODS: [[u16; 36]; 16] = [
    // -8
    [
        907, 856, 808, 762, 720, 678, 640, 604, 570, 538, 508, 480, //
        453, 428, 404, 381, 360, 339, 320, 302, 285, 269, 254, 240, //
        226, 214, 202, 190, 180, 170, 160, 151, 143, 135, 127, 120,
    ],
    // -7
    [
        900, 850, 802, 757, 715, 675, 636, 601, 567, 535, 505, 477, //
        450, 425, 401, 379, 357, 337, 318, 300, 284, 268, 253, 238, //
        225, 212, 200, 189, 179, 169, 159, 150, 142, 134, 126, 119,
    ],
    // -6
    [
        894, 844, 796, 752, 709, 670, 632, 597, 563, 532, 502, 474, //
        447, 422, 398, 376, 355, 335, 316, 298, 282, 266, 251, 237, //
        223, 211, 199, 188, 177, 167, 158, 149, 141, 133, 125, 118,
    ],
    // -5
    [
        887, 838, 791, 746, 704, 665, 628, 592, 559, 528, 498, 470, //
        444, 419, 395, 373, 352, 332, 314, 296, 280, 264, 249, 235, //
        222, 209, 198, 187, 176, 166, 157, 148, 140, 132, 125, 118,
    ],
    // -4
    [
        881, 832, 785, 741, 699, 660, 623, 588, 555, 524, 494, 467, //
        441, 416, 392, 370, 350, 330, 312, 294, 278, 262, 247, 233, //
        220, 208, 196, 185, 175, 165, 156, 147, 139, 131, 123, 117,
    ],
    // -3
    [
        875, 826, 779, 736, 694, 655, 619, 584, 551, 520, 491, 463, //
        437, 413, 390, 368, 347, 328, 309, 292, 276, 260, 245, 232, //
        219, 206, 195, 184, 174, 164, 155, 146, 138, 130, 123, 116,
    ],
    // -2
    [
        868, 820, 774, 730, 689, 651, 614, 580, 547, 516, 487, 460, //
        434, 410, 387, 365, 345, 325, 307, 290, 274, 258, 244, 230, //
        217, 205, 193, 183, 172, 163, 154, 145, 137, 129, 122, 115,
    ],
    // -1
    [
        862, 814, 768, 725, 684, 646, 610, 575, 543, 513, 484, 457, //
        431, 407, 384, 363, 342, 323, 305, 288, 272, 256, 242, 228, //
        216, 203, 192, 181, 171, 161, 152, 144, 136, 128, 121, 114,
    ],
    // +0
    [
        856, 808, 762, 720, 678, 640, 604, 570, 538, 508, 480, 453, //
        428, 404, 381, 360, 339, 320, 302, 285, 269, 254, 240, 226, //
        214, 202, 190, 180, 170, 160, 151, 143, 135, 127, 120, 113,
    ],
    // +1
    [
        850, 802, 757, 715, 674, 637, 601, 567, 535, 505, 477, 450, //
        425, 401, 379, 357, 337, 318, 300, 284, 268, 253, 239, 225, //
        213, 201, 189, 179, 169, 159, 150, 142, 134, 126, 119, 113,
    ],
    // +2
    [
        844, 796, 752, 709, 670, 632, 597, 563, 532, 502, 474, 447, //
        422, 398, 376, 355, 335, 316, 298, 282, 266, 251, 237, 224, //
        211, 199, 188, 177, 167, 158, 149, 141, 133, 125, 118, 112,
    ],
    // +3
    [
        838, 791, 746, 704, 665, 628, 592, 559, 528, 498, 470, 444, //
        419, 395, 373, 352, 332, 314, 296, 280, 264, 249, 235, 222, //
        209, 198, 187, 176, 166, 157, 148, 140, 132, 125, 118, 111,
    ],
    // +4
    [
        832, 785, 741, 699, 660, 623, 588, 555, 524, 495, 467, 441, //
        416, 392, 370, 350, 330, 312, 294, 278, 262, 247, 233, 220, //
        208, 196, 185, 175, 165, 156, 147, 139, 131, 124, 117, 110,
    ],
    // +5
    [
        826, 779, 736, 694, 655, 619, 584, 551, 520, 491, 463, 437, //
        413, 390, 368, 347, 328, 309, 292, 276, 260, 245, 232, 219, //
        206, 195, 184, 174, 164, 155, 146, 138, 130, 123, 116, 109,
    ],
    // +6
    [
        820, 774, 730, 689, 651, 614, 580, 547, 516, 487, 460, 434, //
        410, 387, 365, 345, 325, 307, 290, 274, 258, 244, 230, 217, //
        205, 193, 183, 172, 163, 154, 145, 137, 129, 122, 115, 109,
    ],
    // +7
    [
        814, 768, 725, 684, 646, 610, 575, 543, 513, 484, 457, 431, //
        407, 384, 363, 342, 323, 305, 288, 272, 256, 242, 228, 216, //
        204, 192, 181, 171, 161, 152, 144, 136, 128, 121, 114, 108,
    ],
];

const NOTES_PER_OCTAVE: i8 = 12;

// anything above this gets dropped an octave first
const LAST_INDEX: i8 = 62;

// C-3
const TOP_OCTAVE: usize = 24;

/// Period for a note the way v4 works it out, for normal samples, no extra octaves.
/// `transpose` is instrument + song. Low notes get pushed up by octaves until they fit, very high
/// ones drop one octave, and anything above B-3 folds into the top octave.
pub fn note_period(note: u8, transpose: i8, finetune: i8) -> u16 {
    // i8 on purpose, the player does this in a byte and wraps
    let mut index = (note as i8).wrapping_add(transpose).wrapping_sub(1);
    while index < 0 {
        index += NOTES_PER_OCTAVE;
    }
    if index > LAST_INDEX {
        index -= NOTES_PER_OCTAVE;
    }
    let table = &PERIODS[(finetune.clamp(MIN_FINETUNE, MAX_FINETUNE) - MIN_FINETUNE) as usize];
    let index = index as usize;
    if index < table.len() {
        return table[index];
    }
    // the player's tables repeat the top octave 3 times above B-3. past that it reads into the
    // next finetune's table, we don't copy that
    return table[TOP_OCTAVE + index % NOTES_PER_OCTAVE as usize];
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finetune_zero() {
        assert_eq!(note_period(1, 0, 0), 856);
        assert_eq!(note_period(13, 0, 0), 428);
        assert_eq!(note_period(25, 0, 0), 214);
        assert_eq!(note_period(36, 0, 0), 113);
    }

    #[test]
    fn finetune_table() {
        assert_eq!(note_period(1, 0, -8), 907);
        assert_eq!(note_period(1, 0, 1), 850);
        assert_eq!(note_period(1, 0, 7), 814);
    }

    #[test]
    fn transpose() {
        assert_eq!(note_period(1, 12, 0), 428);
        assert_eq!(note_period(13, -12, 0), 856);
    }

    #[test]
    fn low_notes() {
        assert_eq!(note_period(0, 0, 0), 453);
        assert_eq!(note_period(1, -13, 0), 453);
        assert_eq!(note_period(1, -25, 0), 453);
    }

    #[test]
    fn high_notes_fold() {
        assert_eq!(note_period(37, 0, 0), 214);
        assert_eq!(note_period(48, 0, 0), 113);
        assert_eq!(note_period(61, 0, 0), 214);
    }

    #[test]
    fn past_last_index() {
        // D#-6, looked up as D#-5
        assert_eq!(note_period(64, 0, 0), 180);
    }

    #[test]
    fn transpose_wraps() {
        // 127 + 2 wraps to -127, then octaves up to E-1

        assert_eq!(note_period(127, 2, 0), 678);
    }
}
