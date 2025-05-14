use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

pub struct DonwloadProgress {
    multi_pb: MultiProgress,
    main_pb: ProgressBar,
    chunk_pbs: Vec<ProgressBar>,
}


impl DonwloadProgress {
    pub fn new(total_size: u64, chunks: usize) -> Self {
        let multi_pb = MultiProgress::new();
        let main_pb = multi_pb.add(ProgressBar::new(total_size));
        main_pb.set_style(ProgressStyle::default_bar()
            .template("{msg} [{elapsed_precise}] {wide_bar} {bytes}/{total_bytes} ({eta})")
            .unwrap());
        
        let chunk_pbs = (0..chunks)
            .map(|_| multi_pb.add(ProgressBar::new(0)))
            .collect();

        Self { multi_pb, main_pb, chunk_pbs }
        
    }
}