use std::{ffi::CStr, slice, str};

use block::ConcreteBlock;
use cocoa::{
    base::{id, nil},
    foundation::{NSArray, NSString, NSUInteger, NSInteger},
};
use core_graphics::display::CGDirectDisplayID;
use gpui::{actions, elements::*, keymap::Binding, Menu, MenuItem, mac::dispatcher::dispatch_get_main_queue};
use log::LevelFilter;
use objc::{class, msg_send, sel, sel_impl, declare::ClassDecl, runtime::{Protocol, Object, Sel}};
use simplelog::SimpleLogger;

#[allow(non_upper_case_globals)]
const NSUTF8StringEncoding: NSUInteger = 4;

actions!(capture, [Quit]);

fn main() {
    SimpleLogger::init(LevelFilter::Info, Default::default()).expect("could not initialize logger");

    gpui::App::new(()).unwrap().run(|cx| {
        cx.platform().activate(true);
        cx.add_global_action(quit);

        cx.add_bindings([Binding::new("cmd-q", Quit, None)]);
        cx.set_menus(vec![Menu {
            name: "Zed",
            items: vec![MenuItem::Action {
                name: "Quit",
                action: Box::new(Quit),
            }],
        }]);

        unsafe {
            let block = ConcreteBlock::new(move |content: id, error: id| {
                let displays: id = msg_send![content, displays];
                if let Some(display) = (0..displays.count())
                    .map(|ix| displays.objectAtIndex(ix))
                    .next()
                {
                    let mut decl = ClassDecl::new("CaptureOutput", class!(NSObject)).unwrap();
                    decl.add_protocol(Protocol::get("SCStreamOutput").unwrap());
                    decl.add_method(sel!(stream:didOutputSampleBuffer:ofType:), sample_output as extern "C" fn(&Object, Sel, id, id, NSInteger));
                    let capture_output_class = decl.register();
                    
                    let output: id = msg_send![capture_output_class, alloc];
                    let output: id = msg_send![output, init];
                    
                    let excluded_windows: id = msg_send![class!(NSArray), array];
                    let filter: id = msg_send![class!(SCContentFilter), alloc];
                    let filter: id = msg_send![filter, initWithDisplay: display excludingWindows: excluded_windows];
                    let config: id = msg_send![class!(SCStreamConfiguration), alloc];
                    // Configure the display content width and height.
                    let _: () = msg_send![config, setWidth: 800];
                    let _: () = msg_send![config, setHeight: 600];
                    let _: () = msg_send![config, setMinimumFrameInterval: CMTimeMake(1, 60)];
                    let _: () = msg_send![config, setQueueDepth: 5];
                    
                    let stream: id = msg_send![class!(SCStream), alloc];
                    let stream: id = msg_send![stream, initWithFilter: filter configuration: config delegate: nil];
                    let error: id = nil;
                    let _: () = msg_send![stream, addStreamOutput: output type: 0 sampleHandlerQueue: dispatch_get_main_queue() error: &error];
                    println!("Added stream output... error? {}", string_from_objc(msg_send![error, localizedDescription]));
                    
                    
                    let start_capture_completion = ConcreteBlock::new(move |error: id| {
                        println!("Started capturing... error? {}", string_from_objc(msg_send![error, localizedDescription]));
                        println!("recovery suggestion {}", string_from_objc(msg_send![error, localizedRecoverySuggestion]));
                        println!("failure reason {}", string_from_objc(msg_send![error, localizedFailureReason]));
                        
                        
                    });
                    
                    assert!(!stream.is_null());
                    let _: () = msg_send![stream, startCaptureWithCompletionHandler: start_capture_completion];
                }
            });

            let _: id = msg_send![
                class!(SCShareableContent),
                getShareableContentWithCompletionHandler: block
            ];
        }

        // cx.add_window(Default::default(), |_| ScreenCaptureView);
    });
}

struct ScreenCaptureView;

impl gpui::Entity for ScreenCaptureView {
    type Event = ();
}

impl gpui::View for ScreenCaptureView {
    fn ui_name() -> &'static str {
        "View"
    }

    fn render(&mut self, _: &mut gpui::RenderContext<Self>) -> gpui::ElementBox {
        Empty::new().boxed()
    }
}

pub unsafe fn string_from_objc(string: id) -> String {
    let len = msg_send![string, lengthOfBytesUsingEncoding: NSUTF8StringEncoding];
    let bytes = string.UTF8String() as *const u8;
    str::from_utf8(slice::from_raw_parts(bytes, len))
        .unwrap()
        .to_string()
}

extern "C" fn sample_output(this: &Object, _: Sel, stream: id, buffer: id, kind: NSInteger) {
    println!("sample_output");
}


extern "C" {
    fn CMTimeMake(value: u64, timescale: i32) -> CMTime;
}

#[repr(C)]
struct CMTime {
	value: i64,
	timescale: i32,
	flags: u32,
	epoch: i64,
}


fn quit(_: &Quit, cx: &mut gpui::MutableAppContext) {
    cx.platform().quit();
}
