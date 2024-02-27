//https://github.com/mcbor/lorasim/blob/main/loraDir.py
fn airtime(sf: u32, cr: u32, pl: u32, bw: u32) -> f64 {
    let mut header_disabled = 0_u32; // implicit header disabled (H=0) or not (H=1)
    let mut data_rate_optimization = 0_u32; // low data rate optimization enabled (=1) or not (=0)
    let npream = 8_u32; // number of preamble symbol (12.25 from Utz paper)

    if bw == 125 && (sf == 11 || sf == 12) {
        data_rate_optimization = 1; // low data rate optimization mandated for BW125 with SF11 and SF12
    }
    if sf == 6 {
        header_disabled = 1; // can only have implicit header with SF6
    }

    let tsym = (2.0f64).powi(sf as i32) / bw as f64;
    let tpream = (npream as f64 + 4.25) * tsym;

    let v1 = ((8.0f64 * (pl as f64) - 4.0f64 * (sf as f64) + 44f64 - 20.0f64 * header_disabled as f64)  //28 + 16 = 44(? -->     payloadSymbNB = 8 + max(math.ceil((8.0*pl-4.0*sf+28+16-20*H)/(4.0*(sf-2*DE)))*(cr+4),0))
        / (4.0f64 * ((sf as f64) - 2.0f64 * data_rate_optimization as f64)))
        .ceil()
        * ((cr as f64) + 4.0f64);
    let payload_symb_nb = 8.0 + (if v1 > 0.0 { v1 } else { 0.0 });
    let tpayload = payload_symb_nb * tsym;
    tpream + tpayload
}
