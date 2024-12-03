use std::{
    fs::create_dir_all,
    io::Write,
    process::{Command, Stdio},
    ptr::{from_ref, null, null_mut, NonNull},
};

use block2::StackBlock;
use core_graphics2::{
    color_space::{CGColorRenderingIntent, CGColorSpaceCreateDeviceRGB},
    data_provider::CGDataProviderCreateWithData,
    image::{kCGImageAlphaNone, kCGImageByteOrderDefault, CGImageCreate},
};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
};
use image::{open, DynamicImage};
use itertools::Itertools;
use objc2::{msg_send_id, rc::Retained, ClassType};
use objc2_foundation::{NSArray, NSDictionary, NSError, NSString};
use objc2_vision::{
    VNImageRequestHandler, VNRecognizeTextRequest, VNRecognizedTextObservation, VNRequest,
    VNRequestTextRecognitionLevel,
};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS},
    window::WindowId,
};

fn main() {
    struct App;
    impl ApplicationHandler for App {
        fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

        fn window_event(
            &mut self,
            _event_loop: &ActiveEventLoop,
            _window_id: WindowId,
            _event: WindowEvent,
        ) {
        }
    }
    let event_loop = EventLoopBuilderExtMacOS::with_activation_policy(
        &mut EventLoop::builder(),
        ActivationPolicy::Accessory,
    )
    .build()
    .unwrap();
    let manager = GlobalHotKeyManager::new().unwrap();
    let hotkey = HotKey::new(Some(Modifiers::SHIFT | Modifiers::META), Code::Digit1);
    manager.register(hotkey).unwrap();
    create_dir_all("/tmp/sc").unwrap();
    GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
        if event.state != HotKeyState::Released {
            return;
        }
        Command::new("screencapture")
            .arg("-s")
            .arg("/tmp/sc/capture.png")
            .status()
            .unwrap();
        let image = open("/tmp/sc/capture.png").unwrap();
        get_text(&image, |lines| {
            Command::new("pbcopy")
                .stdin(Stdio::piped())
                .spawn()
                .unwrap()
                .stdin
                .unwrap()
                .write_all(lines.join("\n").as_bytes())
                .unwrap();
        });
    }));
    event_loop.run_app(&mut App).unwrap();
}

fn get_text(image: &DynamicImage, callback: impl Fn(Vec<String>)) {
    unsafe {
        let handler = StackBlock::new(|request: NonNull<VNRequest>, error: *mut NSError| {
            assert!(error.is_null());
            callback(
                request
                    .as_ref()
                    .results()
                    .unwrap()
                    .to_vec()
                    .into_iter()
                    .map(|observation| {
                        Retained::cast::<VNRecognizedTextObservation>(observation.retain())
                            .topCandidates(1)
                            .first()
                            .unwrap()
                            .string()
                            .to_string()
                    })
                    .collect_vec(),
            );
        });
        let request = VNRecognizeTextRequest::initWithCompletionHandler(
            VNRecognizeTextRequest::alloc(),
            from_ref(&*handler) as *mut _,
        );
        request.setRecognitionLevel(VNRequestTextRecognitionLevel::Accurate);
        request.setAutomaticallyDetectsLanguage(true);
        request.setUsesLanguageCorrection(true);
        let image = image.to_rgb8();
        let image = {
            const BITS_PER_COMPONENT: usize = 8;
            const BITS_PER_PIXEL: usize = 24;
            CGImageCreate(
                image.width() as usize,
                image.height() as usize,
                BITS_PER_COMPONENT,
                BITS_PER_PIXEL,
                BITS_PER_PIXEL * image.width() as usize / 8,
                CGColorSpaceCreateDeviceRGB(),
                kCGImageByteOrderDefault | kCGImageAlphaNone,
                {
                    let image = image.into_vec();
                    CGDataProviderCreateWithData(
                        null_mut(),
                        from_ref(image.as_slice()) as *mut _,
                        image.len(),
                        None,
                    )
                },
                null(),
                false,
                CGColorRenderingIntent::Default,
            )
        };
        let options = NSDictionary::<NSString>::new();
        let handler: Retained<VNImageRequestHandler> = msg_send_id![
            VNImageRequestHandler::alloc(),
            initWithCGImage:image
            options:&*options
        ];
        handler
            .performRequests_error(&NSArray::from_slice(&[&*Retained::into_super(
                Retained::into_super(request),
            )]))
            .unwrap();
    }
}
