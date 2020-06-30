use rand::RngCore;
use std::time::Duration;
use trace::trace;

#[tokio::main]
async fn main() {
    env_logger::init();

    let _ = async_early_exit(1000).await;
    // let _ = async_early_exit_expected(1000).await;

    let _ = loop_early_exit(1000);
    // let _ = loop_early_exit_expected(1000);
}

#[trace]
async fn async_early_exit(a: u32) -> Result<u32, String> {
    let mut data = [0u8; 32];
    let mut i = 0;
    loop {
        rand::thread_rng().fill_bytes(&mut data);
        let folded = data.iter().fold(0u32, |total, item| total + *item as u32);
        if folded < a {
            return Ok(folded);
        }
        if i >= 50 {
            return Err("Terribly long".to_string());
        }
        tokio::time::delay_for(Duration::from_millis(100)).await;
        i += 1;
    }
}

// // Expected function for async
// async fn async_early_exit_expected(a: u32) -> Result<u32, String> {
//     log::trace!(">>> loop_early_exit_original\n\ta: {:?}", a);
//
//     let __inner_body__ = move || async move {
//         let mut data = [0u8; 32];
//         let mut i = 0;
//         loop {
//             rand::thread_rng().fill_bytes(&mut data);
//             let folded = data.iter().fold(0u32, |total, item| total + *item as u32);
//             if folded < a {
//                 return Ok(folded);
//             }
//             if i >= 50 {
//                 return Err("Terribly long".to_string());
//             }
//             tokio::time::delay_for(Duration::from_millis(100)).await;
//             i += 1;
//         }
//     };
//
//     let __inner_return_value__ = __inner_body__().await;
//
//     log::trace!(
//         "<<< loop_early_exit_original\n\tres: {:?}",
//         __inner_return_value__
//     );
//
//     __inner_return_value__
// }

//==================================================================================================

#[trace]
fn loop_early_exit(a: u32) -> u32 {
    let mut data = [0u8; 10];
    loop {
        rand::thread_rng().fill_bytes(&mut data);
        let folded = data.iter().fold(0u32, |total, item| total + *item as u32);

        if folded < a {
            return folded;
        }
    }
}

// // Expected function for sync version.
// fn loop_early_exit_expected(a: u32) -> u32 {
//     log::trace!(">>> loop_early_exit_original\n\ta: {:?}", a);
//
//     let __inner_body__ = move || {
//         let mut data = [0u8; 10];
//         loop {
//             rand::thread_rng().fill_bytes(&mut data);
//             let folded = data.iter().fold(0u32, |total, item| total + *item as u32);
//             if folded < a {
//                 return folded;
//             }
//         }
//     };
//     let __inner_return_value__ = __inner_body__();
//
//     log::trace!(
//         "<<< loop_early_exit_original\n\tres: {:?}",
//         __inner_return_value__
//     );
//
//     __inner_return_value__
// }
