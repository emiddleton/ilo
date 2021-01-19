use crossbeam_channel::Sender;
use std::{cmp::Eq, convert::TryFrom, fmt};
use tracing::{event, instrument, Level};

use crate::{gui, transport};

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
enum State {
    Reset,      // 0
    Start,      // 1
    Pixels,     // 2
    PixLru1,    // 3
    PixLru0,    // 4
    PixCode1,   // 5
    PixCode2,   // 6
    PixCode3,   // 7
    PixGrey,    // 8
    PixRgbR,    // 9
    PixRpt,     // 10
    PixRpt1,    // 11
    PixRptStd1, // 12
    PixRptStd2, // 13
    PixRptNStd, // 14
    Cmd,        // 15
    Cmd0,       // 16
    MoveXY0,    // 17
    ExtCmd,     // 18
    CmdX,       // 19
    MoveShortX, // 20
    MoveLongX,  // 21
    BlkRpt,     // 22
    ExtCmd1,    // 23
    Firmware,   // 24
    ExtCmd2,    // 25
    Mode0,      // 26
    Timeout,    // 27
    BlkRpt1,    // 28
    BlkRptStd,  // 29
    BlkRptNStd, // 30
    PixFan,     // 31
    PixCode4,   // 32
    PixDup,     // 33
    BlkDup,     // 34
    PixCode,    // 35
    PixSpec,    // 36
    Exit,       // 37
    Latched,    // 38
    MoveXY1,    // 39
    Mode1,      // 40
    PixRgbG,    // 41
    PixRgbB,    // 42
    Hunt,       // 43
    Print0,     // 44
    Print1,     // 45
    Corp,       // 46
    Mode2,      // 47
}

impl fmt::Display for State {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

//const SIZE_OF_ALL: i32 = 48;

pub enum Error {}

#[derive(Debug)]
pub struct Decoder {
    gui_tx: Sender<gui::Event>,
    transport_tx: Sender<transport::Event>,

    screen_x: i32,
    screen_y: i32,
    scale_x: i32,
    scale_y: i32,

    ts_type: i32,

    //private LocaleTranslator translator = new LocaleTranslator();
    bits_to_read: Vec<i32>,
    next_0: Vec<State>,
    next_1: Vec<State>,

    cc_active: i32,
    cc_color: Vec<i32>,
    cc_usage: Vec<i32>,
    cc_block: Vec<i32>,
    lru_lengths: Vec<i32>,
    getmask: Vec<i32>,
    reversal: Vec<i32>,
    left: Vec<i32>,
    right: Vec<i32>,

    pixel_count: i32,
    size_x: i32,
    size_y: i32,
    y_clipped: i32,
    lastx: i32,
    lasty: i32,
    newx: i32,
    newy: i32,
    color: i32,
    last_color: i32,
    ib_acc: i32,
    ib_bcnt: i32,
    zero_count: i32,
    decoder_state: State,
    next_state: State,
    pixcode: State,
    code: i32,
    block: Vec<i32>,
    red: i32,
    green: i32,
    blue: i32,

    fatal_count: i32,
    printchan: i32,
    printstring: String,
    count_bytes: i64,
    cmd_p_buff: Vec<i32>,
    cmd_p_count: i32,
    cmd_last: i32,
    framerate: i32,

    new_bits: u16,
    debug_lastx: i32,
    debug_lasty: i32,
    debug_show_block: i32,
    timeout_count: i64,

    process_inhibit: bool,
    video_detected: bool,
    color_remap_table: Vec<i32>,
}

// type DvcDecoderState = u8;

pub fn fmt_bits(bits: i32, cnt: i32) -> String {
    if cnt == 0 {
        return String::from("");
    }
    let mut result = String::from("");
    for i in 0..cnt {
        let bit = if (bits >> i) & 0x1 == 1 { '1' } else { '0' };
        result = format!("{}{}", bit, result)
    }
    //format!("0b{}", result);
    result
}

pub trait Decode {
    fn process_dvc(&mut self, param_char: u16) -> bool;
}

impl Decoder {
    //const B: i32 = 0xff000000;
    //const W: i32 = 0xff808080;

    #[instrument]
    pub fn new(gui_tx: Sender<gui::Event>, transport_tx: Sender<transport::Event>) -> Self {
        use State::*;
        Decoder {
            gui_tx,
            transport_tx,
            screen_x: 1,
            screen_y: 1,
            scale_x: 1,
            scale_y: 1,

            ts_type: 0,

            //private LocaleTranslator translator = new LocaleTranslator();
            bits_to_read: vec![
                /* Reset       0 */ 0, // bits
                /* Start       1 */ 1, // bits
                /* Pixels      2 */ 1, // bits
                /* PixLru1     3 */ 1, // bits
                /* PixLru0     4 */ 1, // bits
                /* PixCode1    5 */ 1, // bits
                /* PixCode2    6 */ 2, // bits
                /* PixCode3    7 */ 3, // bits
                /* PixGrey     8 */ 4, // bits
                /* PixRgbR     9 */ 4, // bits
                /* PixRpt     10 */ 1, // bits
                /* PixRpt1    11 */ 1, // bits
                /* PixRptStd1 12 */ 3, // bits
                /* PixRptStd2 13 */ 3, // bits
                /* PixRptNStd 14 */ 8, // bits
                /* Cmd        15 */ 1, // bits
                /* Cmd0       16 */ 1, // bits
                /* MoveXY0    17 */ 7, // bits
                /* ExtCmd     18 */ 1, // bits
                /* CmdX       19 */ 1, // bits
                /* MoveShortX 20 */ 3, // bits
                /* MoveLongX  21 */ 7, // bits
                /* BlkRpt     22 */ 1, // bits
                /* ExtCmd1    23 */ 1, // bits
                /* Firmware   24 */ 8, // bits
                /* ExtCmd2    25 */ 1, // bits
                /* Mode0      26 */ 7, // bits
                /* Timeout    27 */ 0, // bits
                /* BlkRpt1    28 */ 1, // bits
                /* BlkRptStd  29 */ 3, // bits
                /* BlkRptNStd 30 */ 7, // bits
                /* PixFan     31 */ 1, // bits
                /* PixCode4   32 */ 4, // bits
                /* PixDup     33 */ 0, // bits
                /* BlkDup     34 */ 0, // bits
                /* PixCode    35 */ 0, // bits
                /* PixSpec    36 */ 1, // bits
                /* Exit       37 */ 0, // bits
                /* Latched    38 */ 1, // bits
                /* MoveXY1    39 */ 7, // bits
                /* Mode1      40 */ 7, // bits
                /* PixRgbG    41 */ 4, // bits
                /* PixRgbB    42 */ 4, // bits
                /* Hunt       43 */ 1, // bits
                /* Print0     44 */ 8, // bits
                /* Print1     45 */ 8, // bits
                /* Corp       46 */ 1, // bits
                /* Mode2      47 */ 4, // bits
            ],

            next_0: vec![
                /* Reset       0 => */ Start, //       1
                /* Start       1 => */ Pixels, //      2
                /* Pixels      2 => */ PixFan, //     31
                /* PixLru1     3 => */ Pixels, //      2
                /* PixLru0     4 => */ Pixels, //      2
                /* PixCode1    5 => */ PixRpt, //     10
                /* PixCode2    6 => */ PixRpt, //     10
                /* PixCode3    7 => */ PixRpt, //     10
                /* PixGrey     8 => */ PixRpt, //     10
                /* PixRgbR     9 => */ PixRgbG, //    41
                /* PixRpt     10 => */ Pixels, //      2
                /* PixRpt1    11 => */ PixDup, //     33
                /* PixRptStd1 12 => */ Pixels, //      2
                /* PixRptStd2 13 => */ Pixels, //      2
                /* PixRptNStd 14 => */ Pixels, //      2
                /* Cmd        15 => */ Cmd0, //       16
                /* Cmd0       16 => */ CmdX, //       19
                /* MoveXY0    17 => */ MoveXY1, //    39
                /* ExtCmd     18 => */ BlkRpt, //     22
                /* CmdX       19 => */ MoveShortX, // 20
                /* MoveShortX 20 => */ Start, //       1
                /* MoveLongX  21 => */ Start, //       1
                /* BlkRpt     22 => */ BlkDup, //     34
                /* ExtCmd1    23 => */ ExtCmd2, //    25
                /* Firmware   24 => */ Corp, //       46
                /* ExtCmd2    25 => */ Mode0, //      26
                /* Mode0      26 => */ Mode1, //      40
                /* Timeout    27 => */ Start, //       1
                /* BlkRpt1    28 => */ BlkRptStd, //  29
                /* BlkRptStd  29 => */ Start, //       1
                /* BlkRptNStd 30 => */ Start, //       1
                /* PixFan     31 => */ PixSpec, //    36
                /* PixCode4   32 => */ PixRpt, //     10
                /* PixDup     33 => */ Pixels, //      2
                /* BlkDup     34 => */ Start, //       1
                /* PixCode    35 => */ PixCode, //    35
                /* PixSpec    36 => */ PixGrey, //     8
                /* Exit       37 => */ Exit, //       37
                /* Latched    38 => */ Latched, //    38
                /* MoveXY1    39 => */ Start, //       1
                /* Mode1      40 => */ Mode2, //      47
                /* PixRgbG    41 => */ PixRgbB, //    42
                /* PixRgbB    42 => */ PixRpt, //     10
                /* Hunt       43 => */ Hunt, //       43
                /* Print0     44 => */ Print1, //     45
                /* Print1     45 => */ Print1, //     45
                /* Corp       46 => */ Start, //       1
                /* Mode2      47 => */ Start, //       1
            ],
            next_1: vec![
                /* Reset       0 => */ Start, //       1
                /* Start       1 => */ Cmd, //        15
                /* Pixels      2 => */ PixLru1, //     3
                /* PixLru1     3 => */ PixRpt1, //    11
                /* PixLru0     4 => */ PixRpt1, //    11
                /* PixCode1    5 => */ PixRpt, //     10
                /* PixCode2    6 => */ PixRpt, //     10
                /* PixCode3    7 => */ PixRpt, //     10
                /* PixGrey     8 => */ PixRpt, //     10
                /* PixRgbR     9 => */ PixRgbG, //    41
                /* PixRpt     10 => */ PixRpt1, //    11
                /* PixRpt1    11 => */ PixRptStd1, // 12
                /* PixRptStd1 12 => */ Pixels, //      2
                /* PixRptStd2 13 => */ Pixels, //      2
                /* PixRptNStd 14 => */ Pixels, //      2
                /* Cmd        15 => */ MoveXY0, //    17
                /* Cmd0       16 => */ ExtCmd, //     18
                /* MoveXY0    17 => */ MoveXY1, //    39
                /* ExtCmd     18 => */ ExtCmd1, //    23
                /* CmdX       19 => */ MoveLongX, //  21
                /* MoveShortX 20 => */ Start, //       1
                /* MoveLongX  21 => */ Start, //       1
                /* BlkRpt     22 => */ BlkRpt1, //    28
                /* ExtCmd1    23 => */ Firmware, //   24
                /* Firmware   24 => */ Corp, //       46
                /* ExtCmd2    25 => */ Timeout, //    27
                /* Mode0      26 => */ Mode1, //      40
                /* Timeout    27 => */ Start, //       1
                /* BlkRpt1    28 => */ BlkRptNStd, // 30
                /* BlkRptStd  29 => */ Start, //       1
                /* BlkRptNStd 30 => */ Start, //       1
                /* PixFan     31 => */ PixCode, //    35
                /* PixCode4   32 => */ PixRpt, //     10
                /* PixDup     33 => */ Pixels, //      2
                /* BlkDup     34 => */ Start, //       1
                /* PixCode    35 => */ PixCode, //    35
                /* PixSpec    36 => */ PixRgbR, //     9
                /* Exit       37 => */ Exit, //       37
                /* Latched    38 => */ Latched, //    38
                /* MoveXY1    39 => */ Start, //       1
                /* Mode1      40 => */ Mode2, //      47
                /* PixRgbG    41 => */ PixRgbB, //    42
                /* PixRgbB    42 => */ PixRpt, //     10
                /* Hunt       43 => */ Reset, //       0
                /* Print0     44 => */ Print1, //     45
                /* Print1     45 => */ Print1, //     45
                /* Corp       46 => */ Firmware, //   24
                /* Mode2      47 => */ Start, //       1
            ],
            cc_active: 0,
            cc_color: vec![0; 17],
            cc_usage: vec![0; 17],
            cc_block: vec![0; 17],
            lru_lengths: vec![0, 0, 0, 1, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4],
            getmask: vec![0x0, 0x1, 0x3, 0x7, 0xf, 0x1f, 0x3f, 0x7f, 0xff],
            reversal: vec![0; 256],
            left: vec![0; 256],
            right: vec![0; 256],
            pixel_count: 0,
            size_x: 0,
            size_y: 0,
            y_clipped: 0,
            lastx: 0,
            lasty: 0,
            newx: 0,
            newy: 0,
            color: 0,
            last_color: 0,
            ib_acc: 0,
            ib_bcnt: 0,
            zero_count: 0,
            decoder_state: Reset,
            next_state: Reset,
            pixcode: Latched,
            code: 0,
            block: vec![0; 256],
            red: 0,
            green: 0,
            blue: 0,
            fatal_count: 0,
            printchan: 0,
            printstring: "".to_string(),
            count_bytes: 0,
            cmd_p_buff: vec![0; 256],
            cmd_p_count: 0,
            cmd_last: 0,
            framerate: 30,

            new_bits: 0,
            debug_lastx: 0,
            debug_lasty: 0,
            debug_show_block: 0,
            timeout_count: 0,

            process_inhibit: false,
            video_detected: true,
            color_remap_table: vec![0; 4096],
        }
    }
    #[instrument(skip(self))]
    pub fn set_framerate(&mut self, rate: i32) {}
    #[instrument(skip(self))]
    pub fn set_status(&mut self, index: i32, msg: &str) {}

    #[instrument(skip(self))]
    pub fn seize(&mut self) {}

    #[instrument(skip(self))]
    pub fn start_rdp(&mut self) {}
    #[instrument(skip(self))]
    pub fn stop_rdp(&mut self) {}

    #[instrument(skip(self))]
    pub fn change_key(&mut self) {}
    #[instrument(skip(self))]
    pub fn refresh_screen(&mut self) {
        self.transport_tx
            .send(transport::Event::RequestRefresh)
            .unwrap();
    }
    #[instrument(skip(self))]
    fn cache_reset(&mut self) {
        self.cc_active = 0;
    }

    #[instrument(skip(self))]
    fn cache_lru(&mut self, param_int: i32) -> i32 {
        use State::*;

        let mut k = self.cc_active;
        let mut j: usize = 0;
        let mut n: i32 = 0;

        for i in 0..(k as usize) {
            if param_int == self.cc_color[i] {
                j = i;
                n = 1;
                break;
            }
            if self.cc_usage[i] == k - 1 {
                j = i;
            }
        }

        let mut m = self.cc_usage[j];

        if n == 0 {
            if k < 17 {
                j = k as usize;
                m = k;
                k += 1;
                self.cc_active = k;

                self.pixcode = match self.cc_active {
                    0..=1 => Latched,
                    2 => PixLru0,
                    3 => PixCode1,
                    4..=5 => PixCode2,
                    6..=9 => PixCode3,
                    _ => PixCode4,
                };
                event!(
                    Level::DEBUG,
                    self.cc_active,
                    old_next_1 = ?self.next_1[PixFan as usize],
                    next_1 = ?self.pixcode,
                    self_cc_usage_j = self.cc_usage[j],
                );
                self.next_1[PixFan as usize] = self.pixcode;
            }
            self.cc_color[j] = param_int;
        }

        self.cc_block[j] = 1;

        for i in 0..(k as usize) {
            if self.cc_usage[i] < m {
                self.cc_usage[i] += 1;
            }
        }

        self.cc_usage[j] = 0;
        n
    }

    #[instrument(skip(self))]
    fn cache_find(&mut self, param_int: i32) -> i32 {
        let i = self.cc_active;
        for j in 0..(i as usize) {
            if param_int == self.cc_usage[j] {
                let m = self.cc_color[j];
                let k = j;

                for j in 0..(i as usize) {
                    if self.cc_usage[j] < param_int {
                        self.cc_usage[j] += 1;
                    }
                }
                self.cc_usage[k] = 0;
                self.cc_block[k] = 1;
                event!(Level::TRACE, color=m, ?self.cc_usage, ?self.cc_block);
                return m;
            }
        }
        -1
    }

    #[instrument(skip(self))]
    fn cache_prune(&mut self) {
        use State::*;

        let mut j = self.cc_active as usize;
        let mut i: usize = 0;
        while i < j {
            let k = self.cc_block[i];
            if k == 0 {
                j -= 1;
                self.cc_block[i] = self.cc_block[j];
                self.cc_color[i] = self.cc_color[j];
                self.cc_usage[i] = self.cc_usage[j];
            } else {
                self.cc_block[i] -= 1;
                i += 1;
            }
        }

        self.cc_active = j as i32;
        self.pixcode = match self.cc_active {
            0..=1 => Latched,
            2 => PixLru0,
            3 => PixCode1,
            4..=5 => PixCode2,
            6..=9 => PixCode3,
            _ => PixCode4,
        };
        event!(
            Level::DEBUG,
            self.cc_active,
            old_next_1 = ?self.next_1[PixFan as usize],
            next_1 = ?self.pixcode,
        );
        self.next_1[PixFan as usize] = self.pixcode;
    }

    #[instrument(skip(self))]
    fn next_block(&mut self, param_int: i32) {
        let mut k = 1;
        if !self.video_detected {
            k = 0;
        }

        if self.pixel_count != 0 && (self.y_clipped > 0) && (self.lasty == self.size_y) {
            let m = self.color_remap_table[0];
            for j in (self.y_clipped as usize)..256 {
                self.block[j] = m;
            }
        }
        self.pixel_count = 0;
        self.next_state = State::Start;

        let mut i = self.lastx * 16;
        let j = self.lasty * 16;

        let mut pos = param_int;
        while pos != 0 {
            if k != 0 {
                self.gui_tx
                    .send(gui::Event::PasteArray {
                        block: self.block.clone(),
                        i,
                        j,
                    })
                    .unwrap();
            }

            self.lastx += 1;
            i += 16;

            if self.lastx >= self.size_x {
                break;
            }
            pos -= 1;
        }
    }

    #[instrument(skip(self))]
    fn init_reversal(&mut self) {
        for i in 0..256 {
            let mut i1 = 8;
            let mut n = 8;
            let mut k = i;
            let mut m = 0;
            for j in 0..8 {
                m <<= 1;
                if (k & 0x1) == 1 {
                    if i1 > j {
                        i1 = j;
                    }
                    m |= 0x1;
                    n = 7 - j;
                }
                k >>= 1;
            }
            self.reversal[i] = m;
            self.right[i] = i1;
            self.left[i] = n;
        }
        event!(
            Level::TRACE,
            ?self.reversal,
            ?self.right,
            ?self.left
        )
    }

    #[instrument(skip(self))]
    fn add_bits(&mut self, param_char: u16) -> i32 {
        use State::*;
        event!(
            Level::TRACE,
            self.zero_count,
            right_param_char = self.right[param_char as usize]
        );
        self.zero_count += self.right[param_char as usize];

        let i: i32 = param_char as i32;
        self.ib_acc |= i << self.ib_bcnt;
        self.ib_bcnt += 8;

        event!(
            Level::TRACE,
            bits_added = ?fmt_bits(param_char as i32, 8),
            current_bits = ?fmt_bits(self.ib_acc,self.ib_bcnt),
        );

        if self.zero_count > 30 {
            match self.decoder_state {
                Latched if self.fatal_count < 40 && self.fatal_count > 0 => event!(
                    Level::INFO,
                    ?self.decoder_state,
                    self.fatal_count,
                    "reset caused a false alarm"
                ),
                _ => event!(
                    Level::INFO,
                    ?self.decoder_state,
                    "Reset sequence detected at {} byte",
                    self.count_bytes
                ),
            }
            self.next_state = Hunt;
            self.decoder_state = Hunt;
            return 4;
        }
        if param_char != 0 {
            self.zero_count = self.left[param_char as usize];
            event!(Level::TRACE, self.zero_count);
        }
        0
    }

    #[instrument(skip(self))]
    fn get_bits(&mut self, param_int: i32) -> i32 {
        if param_int == 1 {
            self.code = self.ib_acc & 0x1;
            self.ib_acc >>= 1;
            self.ib_bcnt -= 1;
            event!(
                Level::TRACE,
                code = ?fmt_bits(self.code, param_int),
                remaining = ?fmt_bits(self.ib_acc, self.ib_bcnt),
            );
            return 0;
        }

        if param_int == 0 {
            return 0;
        }

        let mut i = self.ib_acc & self.getmask[param_int as usize];
        self.ib_bcnt -= param_int;
        self.ib_acc >>= param_int;

        event!(Level::TRACE, reversal_index = i);
        i = self.reversal[i as usize];
        event!(Level::TRACE, reversal = i);
        i >>= 8 - param_int;
        self.code = i;
        event!(
            Level::TRACE,
            code = ?fmt_bits(self.code, param_int),
            remaining = ?fmt_bits(self.ib_acc, self.ib_bcnt),
        );
        0
    }

    #[instrument(skip(self))]
    fn bits_to_read(&self, state: &State) -> i32 {
        use State::*;

        match state {
            Reset | Timeout | PixDup | BlkDup | PixCode | Exit => 0,
            Start | Pixels | PixLru1 | PixLru0 | PixCode1 | PixRpt | PixRpt1 | Cmd | Cmd0
            | ExtCmd | CmdX | BlkRpt | ExtCmd1 | ExtCmd2 | BlkRpt1 | PixFan | PixSpec | Latched
            | Hunt | Corp => 1,
            PixCode2 => 2,
            PixCode3 | PixRptStd1 | PixRptStd2 | MoveShortX | BlkRptStd => 3,
            PixGrey | PixRgbR | PixCode4 | PixRgbG | PixRgbB | Mode2 => 4,
            MoveXY0 | MoveLongX | Mode0 | BlkRptNStd | MoveXY1 | Mode1 => 7,
            PixRptNStd | Firmware | Print0 | Print1 => 8,
        }
    }

    #[instrument(skip(self))]
    fn process_bits(&mut self, param_char: u16) -> i32 {
        let mut m = 0;
        self.add_bits(param_char);
        self.new_bits = param_char;
        self.count_bytes += 1;
        let mut k;

        'label2353: while m == 0 {
            event!(Level::TRACE, current_state = ?self.decoder_state);

            k = self.bits_to_read(&self.decoder_state);
            event!(Level::TRACE, needed_bits = k);

            if k > self.ib_bcnt {
                m = 0;
                event!(
                    Level::TRACE,
                    required_bits = k,
                    available_bits = self.ib_bcnt,
                    "not enough bits for current command"
                );
                break;
            }

            let mut _i = self.get_bits(k);
            self.next_state = if self.code == 0 {
                self.next_0[self.decoder_state as usize]
            } else {
                self.next_1[self.decoder_state as usize]
            };

            event!(
                Level::DEBUG,
                current_state = ?self.decoder_state,
                code = ?fmt_bits(self.code,k),
                next_state = ?self.next_state,
            );

            use State::*;
            match self.decoder_state {
                PixLru1 | PixLru0 | PixCode1 | PixCode2 | PixCode3 | PixCode4 => {
                    event!(
                        Level::DEBUG,
                        self.cc_active,
                        ?self.decoder_state
                    );
                    if self.cc_active == 1 {
                        self.code = self.cc_usage[0];
                    } else if self.decoder_state == PixLru0 {
                        self.code = 0;
                    } else if self.decoder_state == PixLru1 {
                        self.code = 1;
                    } else if self.code != 0 {
                        self.code += 1;
                    }
                    event!(Level::DEBUG, self.code);
                    self.color = self.cache_find(self.code);
                    match self.color {
                        -1 => {
                            self.next_state = Latched;
                            event!(
                                Level::WARN,
                                ?self.next_state,
                                "could not find color LRU {}, cache has {} colors",
                                self.code,
                                self.cc_active
                            );
                        }
                        color => {
                            self.color = color;
                            self.last_color = self.color_remap_table[color as usize];
                            if self.pixel_count < 256 {
                                self.block[self.pixel_count as usize] = self.last_color;
                                event!(
                                    Level::TRACE,
                                    block_pixel_count = self.block[self.pixel_count as usize],
                                    self.pixel_count,
                                    self.last_color,
                                    "setting pixel color"
                                );
                                self.pixel_count += 1;
                            } else {
                                self.next_state = Latched;
                                event!(Level::WARN, self.pixel_count, ?self.next_state, "too many block0");
                                break 'label2353;
                            }
                        }
                    }
                }
                PixRptStd1 => {
                    event!(Level::DEBUG, self.code);
                    if self.code == 7 {
                        self.next_state = PixRptNStd;
                    } else if self.code == 6 {
                        self.next_state = PixRptStd2;
                    } else {
                        self.code += 2;
                        for _ in 0..self.code {
                            if self.pixel_count < 256 {
                                self.block[self.pixel_count as usize] = self.last_color;
                                event!(
                                    Level::TRACE,
                                    self.pixel_count,
                                    self.last_color,
                                    "filling block at"
                                );
                            } else {
                                event!(
                                    Level::ERROR,
                                    self.pixel_count,
                                    "too many pixels in a block2"
                                );
                                self.next_state = Latched;
                                break;
                            }
                            self.pixel_count += 1;
                        }
                    }
                }
                PixRptStd2 | PixRptNStd => {
                    if self.decoder_state == PixRptStd2 {
                        self.code += 8;
                    }
                    if self.decoder_state == PixRptNStd && self.code < 16 {
                        event!(Level::WARN, "non-std repeat misused");
                    }
                    for _ in 0..self.code {
                        if self.pixel_count < 256 {
                            self.block[self.pixel_count as usize] = self.last_color;
                            event!(
                                Level::TRACE,
                                self.pixel_count,
                                self.last_color,
                                "filling block at"
                            );
                        } else {
                            event!(Level::ERROR, "too many pixels in a block3");
                            self.next_state = Latched;
                            break;
                        }
                        self.pixel_count += 1;
                    }
                }
                PixDup => {
                    if self.pixel_count < 256 {
                        self.block[self.pixel_count as usize] = self.last_color;
                    } else {
                        event!(Level::WARN, "too many pixels in a block4");
                        self.next_state = Latched;
                        break 'label2353;
                    }
                    self.pixel_count += 1;
                }
                Start | Pixels | PixRpt | PixRpt1 | BlkRpt | BlkRpt1 | PixFan | PixSpec => {}
                PixCode => self.next_state = self.pixcode,
                PixRgbR => self.red = self.code << 8,
                PixRgbG => self.green = self.code << 4,
                PixGrey | PixRgbB => {
                    // copy same value into RGB for Grey
                    if self.decoder_state == PixGrey {
                        self.red = self.code << 8;
                        self.green = self.code << 4;
                        event!(Level::TRACE, self.red, self.green);
                    }
                    self.blue = self.code;

                    self.color = self.red | self.green | self.blue;
                    if 0 != self.cache_lru(self.color) {
                        if self.count_bytes > 6 {
                            event!(Level::WARN, self.color, "unexpected hit");
                        } else {
                            event!(Level::INFO, self.color, "possible reset underway");
                        }
                        self.next_state = Latched;
                    } else {
                        self.last_color = self.color_remap_table[self.color as usize];
                        if self.pixel_count < 256 {
                            self.block[self.pixel_count as usize] = self.last_color;
                        } else {
                            event!(Level::WARN, "too many block1");
                            self.next_state = Latched;
                            break 'label2353;
                        }
                        self.pixel_count += 1;
                    }
                }
                MoveXY0 | Mode0 => {
                    self.newx = self.code;
                    event!(Level::TRACE, self.newx);
                    if self.decoder_state == MoveXY0 && self.newx > self.size_x {
                        event!(
                            Level::ERROR,
                            byte_count = self.count_bytes,
                            self.newx,
                            "MoveXY moves x beyond screen"
                        );
                        self.newx = 0;
                    }
                }
                MoveXY1 => {
                    self.newy = self.code & 0x7F;

                    self.lastx = self.newx;
                    self.lasty = self.newy;

                    if self.lasty > self.size_y {
                        event!(
                            Level::ERROR,
                            byte_count = self.count_bytes,
                            self.lasty,
                            "MoveXY moves y beyond screen"
                        );
                        self.lasty = 0;
                    }
                    self.gui_tx.send(gui::Event::RepaintIt).unwrap();
                }
                MoveShortX | MoveLongX => {
                    if self.decoder_state == MoveShortX {
                        self.code = self.lastx + self.code + 1;
                        if self.code > self.size_x {
                            event!(
                                Level::DEBUG,
                                self.code,
                                self.lastx,
                                byte_count = self.count_bytes,
                                "short x moves beyond screen"
                            );
                        }
                    }
                    self.lastx = self.code & 0x7F;
                    if self.lastx > self.size_x {
                        event!(
                            Level::DEBUG,
                            self.lastx,
                            byte_count = self.count_bytes,
                            "long x moves beyond screen",
                        );
                        self.lastx = 0;
                    }
                }
                Timeout => {
                    event!(Level::TRACE, self.timeout_count, self.count_bytes);
                    if self.timeout_count == self.count_bytes - 1 {
                        event!(
                            Level::WARN,
                            self.count_bytes,
                            remaining_bits = (self.ib_bcnt & 0x7),
                            "double timeout at"
                        );
                        self.next_state = Latched;
                    }
                    if (self.ib_bcnt & 0x7) != 0 {
                        self.get_bits(self.ib_bcnt & 0x7);
                    }
                    self.timeout_count = self.count_bytes;
                    event!(Level::TRACE, self.timeout_count);
                    self.gui_tx.send(gui::Event::RepaintIt).unwrap();
                }
                Firmware => {
                    event!(
                        Level::TRACE,
                        self.cmd_p_count,
                        ?self.cmd_p_buff,
                        self.cmd_last
                    );
                    if self.cmd_p_count != 0 {
                        self.cmd_p_buff[(self.cmd_p_count - 1) as usize] = self.cmd_last;
                    }
                    self.cmd_p_count += 1;
                    self.cmd_last = self.code;
                    event!(Level::TRACE, self.cmd_last);
                }
                Corp => {
                    if self.code == 0 {
                        match self.cmd_last {
                            1 => {
                                self.next_state = Exit;
                                event!(Level::TRACE, ?self.next_state);
                            }
                            2 => {
                                self.next_state = Print0;
                                event!(Level::TRACE, ?self.next_state);
                            }
                            3 => {
                                if self.cmd_p_count != 0 {
                                    self.set_framerate(self.cmd_p_buff[0]);
                                    event!(Level::TRACE, set_framerate=?self.cmd_p_buff[0]);
                                } else {
                                    self.set_framerate(0);
                                    event!(Level::TRACE, set_framerate = 0);
                                }
                            }
                            4 | 5 => {}
                            6 => {
                                event!(Level::DEBUG, "sending -> ShowText Video suspended");
                                self.gui_tx
                                    .send(gui::Event::ShowText {
                                        text: "Video suspended".to_string(),
                                    })
                                    .unwrap();

                                self.set_status(2, "Video_suspended");
                                self.screen_x = 640;
                                self.screen_y = 100;
                                event!(
                                    Level::INFO,
                                    self.screen_x,
                                    self.screen_y,
                                    "Video suspended"
                                );
                            }
                            7 => {
                                self.ts_type = self.cmd_p_buff[0];
                                self.start_rdp();
                                event!(Level::INFO, self.ts_type, "Start RDP");
                            }
                            8 => {
                                self.stop_rdp();
                                event!(Level::INFO, "Stop RDP");
                            }
                            9 => {
                                if (self.ib_bcnt & 0x7) != 0 {
                                    self.get_bits(self.ib_bcnt & 0x7);
                                }
                                self.change_key();
                                event!(Level::INFO, "Change Key");
                            }
                            10 => {
                                self.seize();
                                event!(Level::INFO, "Session Seized");
                            }
                            _ => println!("dvc: unknown firmware command {}", self.cmd_last),
                        }
                        self.cmd_p_count = 0;
                    }
                }
                Print0 => {
                    self.printchan = self.code;
                    self.printstring = "".to_string();
                    event!(Level::TRACE, self.printchan, ?self.printstring);
                }
                Print1 => {
                    if self.code != 0 {
                        let c = vec![u8::try_from(self.code).unwrap()];
                        let c = String::from_utf8(c).unwrap();
                        self.printstring += c.as_str();
                        event!(Level::DEBUG, ?self.printstring);
                    } else {
                        match self.printchan {
                            1 | 2 => {
                                self.set_status(2 + self.printchan, &self.printstring.clone());
                                event!(Level::DEBUG, ?self.printstring, "Set Status");
                            }
                            3 => {
                                println!("{}", &self.printstring);
                                event!(Level::DEBUG, ?self.printstring, "Stdout");
                            }
                            4 => {
                                self.gui_tx
                                    .send(gui::Event::ShowText {
                                        text: self.printstring.clone(),
                                    })
                                    .unwrap();
                                event!(Level::DEBUG, ?self.printstring, "Show Text");
                            }
                            _ => panic!("unknown printchan {}", self.printchan),
                        }
                        self.next_state = Start;
                    }
                }
                Cmd | Cmd0 | ExtCmd | CmdX | ExtCmd1 | ExtCmd2 => {}
                Reset => {
                    self.cache_reset();
                    self.pixel_count = 0;
                    self.lastx = 0;
                    self.lasty = 0;
                    self.red = 0;
                    self.green = 0;
                    self.blue = 0;
                    self.fatal_count = 0;
                    self.timeout_count = -1;
                    self.cmd_p_count = 0;
                }
                Latched => {
                    if self.fatal_count == 0 {
                        self.debug_lastx = self.lastx;
                        self.debug_lasty = self.lasty;
                        self.debug_show_block = 1;
                    }
                    if self.fatal_count == 40 {
                        event!(
                            Level::WARN,
                            byte_count = self.count_bytes,
                            current_block_at = ?(self.lastx, self.lasty),
                            "Latched"
                        );
                    }
                    if self.fatal_count == 11680 {
                        self.transport_tx
                            .send(transport::Event::RequestRefresh)
                            .unwrap();
                    }
                    self.fatal_count += 1;
                    if self.fatal_count == 120000 {
                        event!(Level::WARN, "Requesting refresh1");
                        self.transport_tx
                            .send(transport::Event::RequestRefresh)
                            .unwrap();
                    }
                    if self.fatal_count == 12000000 {
                        event!(Level::WARN, "Requesting refresh2");
                        self.transport_tx
                            .send(transport::Event::RequestRefresh)
                            .unwrap();
                        self.fatal_count = 41;
                    }
                }
                BlkDup => self.next_block(1),
                BlkRptStd | BlkRptNStd => {
                    if self.decoder_state == BlkRptStd {
                        self.code += 2
                    }
                    self.next_block(self.code)
                }
                Mode1 => {
                    self.size_x = self.newx;
                    self.size_y = self.code;
                    event!(Level::TRACE, self.size_x, self.size_y);
                }
                Mode2 => {
                    self.lastx = 0;
                    self.lasty = 0;
                    self.pixel_count = 0;
                    self.cache_reset();
                    self.scale_x = 1;
                    self.scale_y = 1;
                    self.screen_x = self.size_x * 16;
                    self.screen_y = self.size_y * 16 + self.code;

                    self.video_detected = !((self.screen_x == 0) || (self.screen_y == 0));
                    self.y_clipped = if self.code > 0 {
                        256 - 16 * self.code
                    } else {
                        0
                    };
                    event!(
                        Level::DEBUG,
                        self.video_detected,
                        self.screen_x,
                        self.screen_y,
                        self.y_clipped
                    );
                    if !self.video_detected {
                        self.gui_tx
                            .send(gui::Event::ShowText {
                                text: String::from("No Video"),
                            })
                            .unwrap();
                        self.set_status(2, "No Video");
                        self.screen_x = 640;
                        self.screen_y = 100;
                    } else {
                        self.gui_tx
                            .send(gui::Event::ShowText {
                                text: format!(" Video: {}x{}", self.screen_x, self.screen_y),
                            })
                            .unwrap();
                        self.gui_tx
                            .send(gui::Event::SetAbsDimensions {
                                width: self.screen_x,
                                height: self.screen_y,
                            })
                            .unwrap();
                        self.set_status(2, &format!(" Video:{}x{}", self.screen_x, self.screen_y));
                    }
                }
                Hunt => {
                    if self.next_state != self.decoder_state {
                        event!(Level::DEBUG, "resetting");
                        self.ib_bcnt = 0;
                        self.ib_acc = 0;
                        self.zero_count = 0;
                        self.count_bytes = 0;
                    }
                }
                Exit => return 1,
            }

            if self.next_state == Pixels && self.pixel_count == 256 {
                event!(Level::TRACE, "moving to next block");
                self.next_block(1);
                self.cache_prune();
            }

            if self.decoder_state == self.next_state
                && self.decoder_state != Print1
                && self.decoder_state != Latched
                && self.decoder_state != Hunt
            {
                event!(Level::ERROR, "Machine hung in state {}", self.decoder_state);
                m = 6;
            } else {
                event!(Level::TRACE, next_state=?self.next_state, "change to");
                self.decoder_state = self.next_state;
            }
        }
        m
    }
}

impl Decode for Decoder {
    #[instrument(skip(self))]
    fn process_dvc(&mut self, param_char: u16) -> bool {
        if self.reversal[0xFF] == 0 {
            self.init_reversal();
            self.cache_reset();
            self.decoder_state = State::Reset;
            self.next_state = State::Reset;
            self.zero_count = 0;
            self.ib_acc = 0;
            self.ib_bcnt = 0;
            for j in 0..4096i32 {
                self.color_remap_table[j as usize] =
                    (j & 0xF00) * 0x1100 + (j & 0xF0) * 0x110 + (j & 0xF) * 0x11;
            }
        }

        let i;
        if !self.process_inhibit {
            i = self.process_bits(param_char);
        } else {
            i = 0;
        }
        let ret;
        if i == 0 {
            ret = true;
        } else {
            event!(Level::ERROR, "Exit from DVC mode status = {}", i);
            event!(
                Level::ERROR,
                "Current block at {} {}",
                self.lastx,
                self.lasty
            );
            event!(Level::ERROR, "Byte count {}", self.count_bytes);
            ret = true;

            self.decoder_state = State::Latched;
            self.next_state = State::Latched;

            self.fatal_count = 0;
        }
        ret
    }
}

#[cfg(test)]
mod test {
    /*
    use crypto::{
        buffer,
        buffer::{
            BufferResult::{BufferOverflow, BufferUnderflow},
            ReadBuffer, WriteBuffer,
        },
        rc4::Rc4,
        symmetriccipher::Decryptor,
    };
    */
    use super::*;
    use crossbeam_channel::unbounded;
    //use nom::number::streaming::{be_u16, be_u64};
    //use nom::{do_parse, named};
    //use pcap_parser::data::get_packetdata;
    use crate::{gui, transport};
    //use pcap_parser::*;
    //use pnet::packet::ip::IpNextHeaderProtocols;
    use tracing::Level;
    use tracing_subscriber::FmtSubscriber;

    //static TEST_FILE: &[u8] = include_bytes!("../../ilo-test-2.pcap");

    /*
    fn find_enc_start(next_byte: &u8, enc_header_pos: u32) -> u32 {
        match next_byte {
            0x1b => 1,
            b'[' if enc_header_pos == 1 => 2,
            b'R' | b'r' if enc_header_pos == 2 => {
                if b'R' == *next_byte {
                    3
                } else {
                    panic!("unencrypted");
                }
            }
            _ => 0,
        }
    }
    */

    #[test]
    fn test_decode() {
        let subscriber = FmtSubscriber::builder()
            .with_max_level(Level::ERROR)
            .finish();
        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");

        let (transport_tx, _transport_rx) = unbounded::<transport::Event>();
        let (gui_tx, gui_rx) = unbounded::<gui::Event>();

        let mut decoder = Decoder::new(gui_tx, transport_tx.clone());
        //decoder.debug_msgs = true;

        //let enc_header_pos = 0;
        use std::fs::File;
        use std::io::Read;
        let f = File::open("./decrypted_data.dat").unwrap();
        for cl in f.bytes() {
            let cl = cl.unwrap();
            println!("processing {}", cl);
            let dvc_mode = decoder.process_dvc(cl as u16);
            if !dvc_mode {
                panic!("finished encoded section");
            }
        }
        let _ = crate::gui::handle(gui_rx, transport_tx);
    }

    /*
    #[tokio::test]
    async fn test_pcap() {
        /*
        let session_decrypt_key = vec![
            171, 53, 14, 168, 45, 79, 14, 182, 34, 50, 68, 122, 122, 215, 88,
            176, //195, 125, 64, 185, 253, 195, 248, 137, 55, 44, 155, 79, 189, 171, 58, 170,
        ];
        */
        let session_decrypt_key = vec![
            77, 10, 180, 36, 170, 177, 40, 110, 63, 2, 36, 110, 240, 247, 103, 225,
        ];
        let decrypt_key: [u8; 16] =
            md5::compute([&session_decrypt_key[..], &[0; 16][..]].concat()).0;
        let mut rc4_decrypter = Rc4::new(&decrypt_key);
        let cap = PcapCapture::from_file(&TEST_FILE).unwrap();
        let mut enc_header_pos: u32 = 0;
        let mut buffer: [u8; 1024] = [0; 1024];
        let (tokio_tx, tokio_rx) = mpsc::channel::<NetworkEvent>();
        let (gui_tx, gui_rx) = mpsc::channel::<GuiEvent>();
        let mut decoder = super::DvcDecoder::new(tokio_tx); //, gui_rx);
        decoder.debug_msgs = true;
        for block in cap.iter() {
            match block {
                PcapBlock::LegacyHeader(b) => panic!("{:?}", b),
                PcapBlock::Legacy(b) => {
                    if let Some(eth_pkt) = EthernetPacket::new(b.data) {
                        if let Some(ipv4_pkt) = Ipv4Packet::new(eth_pkt.payload()) {
                            match ipv4_pkt.get_next_level_protocol() {
                                IpNextHeaderProtocols::Tcp => {
                                    if let Some(tcp_pkt) = TcpPacket::new(ipv4_pkt.payload()) {
                                        if tcp_pkt.get_destination() == 23 {
                                            let data: Vec<u8> = tcp_pkt.payload().into();
                                            if data.len() > 0 {
                                                //println!("\n\n{:#?}", &eth_pkt);
                                                //println!("{:#?}", &ipv4_pkt);
                                                //println!("{:#?}", &tcp_pkt);
                                                //println!("\n\nclient ->");
                                                //println!("{:02x?}", data);
                                            }
                                        } else if tcp_pkt.get_source() == 23 {
                                            let data: Vec<u8> = tcp_pkt.payload().into();
                                            if data.len() > 0 {
                                                //println!("\n\n{:#?}", &eth_pkt);
                                                //println!("{:#?}", &ipv4_pkt);
                                                //println!("{:#?}", &tcp_pkt);
                                                //println!("{:02x?}\n", data);
                                                //println!("\n\n<- server");
                                                //println!("{:02x?}\n", data);
                                                let mut buffer_iter = data.into_iter();
                                                while let Some(next_byte) = buffer_iter.next() {
                                                    if enc_header_pos == 3 {
                                                        let buff: Vec<u8> = buffer_iter.collect();
                                                        //let buff = vec![buffer_iter.next().unwrap()];

                                                        let mut decrypted = Vec::<u8>::new();
                                                        let mut read_buffer =
                                                            buffer::RefReadBuffer::new(&buff);
                                                        let mut buffer = [0; 1024];
                                                        let mut write_buffer =
                                                            buffer::RefWriteBuffer::new(
                                                                &mut buffer,
                                                            );
                                                        loop {
                                                            let rslt = rc4_decrypter
                                                                .decrypt(
                                                                    &mut read_buffer,
                                                                    &mut write_buffer,
                                                                    true,
                                                                )
                                                                .unwrap();
                                                            decrypted.extend(
                                                                write_buffer
                                                                    .take_read_buffer()
                                                                    .take_remaining()
                                                                    .iter()
                                                                    .map(|&i| i),
                                                            );
                                                            match rslt {
                                                                BufferUnderflow => {
                                                                    //println!("buffer underflow");
                                                                    break;
                                                                }
                                                                BufferOverflow => {
                                                                    //println!("buffer overflow keep loading...")
                                                                }
                                                            }
                                                        }
                                                        //println!("{:02x?}", decrypted);
                                                        for cl in decrypted.into_iter() {
                                                            //println!("processing {:#X}", cl);
                                                            let dvc_mode =
                                                                decoder.process_dvc(cl as u16);
                                                            if !dvc_mode {
                                                                enc_header_pos = 0;
                                                            }
                                                        }
                                                        break;
                                                    } else {
                                                        enc_header_pos = find_enc_start(
                                                            &next_byte,
                                                            enc_header_pos,
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                _ => {}
                            }
                            //if let Some()
                        }
                    }
                    //println!("PacketData: {:?}\n", Ipv4Packet::new(b.data));
                    //assert_eq!(b.caplen, 90);
                }
                PcapBlock::NG(b) => match b {
                    Block::SectionHeader(s) => println!("{:?}", s),
                    Block::InterfaceDescription(s) => println!("{:?}", s),
                    _ => (),
                    /*
                    Block::EnhancedPacket(s) => println!("{:?}", s),
                    Block::SimplePacket(s) => println!("{:?}", s),
                    Block::NameResolution(s) => println!("{:?}", s),
                    Block::InterfaceStatistics(s) => println!("{:?}", s),
                    Block::SystemdJournalExport(s) => println!("{:?}", s),
                    Block::DecryptionSecrets(s) => println!("{:?}", s),
                    Block::Custom(s) => println!("{:?}", s),
                    Block::Unknown(s) => println!("{:?}", s),
                    */
                },
            }
        }
        crate::handle_graphics(gui_tx, tokio_rx);
        ()
    }
    */
}
