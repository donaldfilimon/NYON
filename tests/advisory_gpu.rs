use std::time::{Duration, Instant};

use nyon::{
    advisory::gpu::{FEATURES_BUFFER_SIZE, GpuAdvisory, SCORES_BUFFER_SIZE, WEIGHTS_BUFFER_SIZE},
    advisory::{AdvisoryController, AdvisoryTrigger, CompletionDisposition},
    game::model::{Campaign, DEFAULT_SEED, RulesV1},
};

#[test]
fn gpu_scores_match_cpu_or_reports_a_real_adapter_skip() {
    assert_eq!(FEATURES_BUFFER_SIZE, 336);
    assert_eq!(WEIGHTS_BUFFER_SIZE, 228);
    assert_eq!(SCORES_BUFFER_SIZE, 28);

    pollster::block_on(async {
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(descriptor);
        let adapter = match instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                force_fallback_adapter: false,
                compatible_surface: None,
                apply_limit_buckets: false,
            })
            .await
        {
            Ok(adapter) => adapter,
            Err(error) => {
                eprintln!("SKIP: no headless WebGPU adapter is available: {error}");
                return;
            }
        };
        if !GpuAdvisory::adapter_supported(&adapter) {
            eprintln!("SKIP: adapter lacks DownlevelFlags::COMPUTE_SHADERS");
            return;
        }
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("advisory integration test device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .expect("supported adapter must provide the baseline test device");
        let mut gpu = GpuAdvisory::new(&adapter, &device, &queue)
            .expect("advisory GPU initialization must succeed")
            .expect("compute support was checked above");
        let campaign = Campaign::new(DEFAULT_SEED, RulesV1::default());
        let mut controller = AdvisoryController::new(1, true);
        let request = controller
            .request_if_due(&campaign, 0x1234, AdvisoryTrigger::Initialization)
            .expect("initial request is required")
            .gpu_request
            .expect("supported GPU receives the request");
        gpu.submit(&request)
            .expect("request submission must succeed");
        assert!(gpu.has_in_flight_request());
        assert!(matches!(
            gpu.submit(&request),
            Err(nyon::advisory::gpu::GpuAdvisoryError::Busy)
        ));

        let deadline = Instant::now() + Duration::from_secs(10);
        let completion = loop {
            if let Some(completion) = gpu.poll().expect("nonblocking device poll must succeed") {
                break completion;
            }
            assert!(
                Instant::now() < deadline,
                "GPU readback did not complete within 10 seconds"
            );
            std::thread::yield_now();
        };
        assert_eq!(completion.metadata, request.metadata);
        let outcome = controller.complete_gpu(completion.metadata, completion.result);
        assert_eq!(outcome.disposition, CompletionDisposition::Accepted);
    });
}
