//! Sequential outlier detection and removal using Hampel identifiers.
//! 
//! It supports `f32` and `f64`.
//! 
//! # Example
//! 
//! ```rust
//! use hampel::Window;
//! 
//! fn main() {
//!     // Window size: 5 (>= 3)
//!     // Initialization value of window: 0.0
//!     // Threshold: Median of the window ±3σ.
//!     let mut filter = Window::<f64, 5>::new(0.0, 3.0);
//!     
//!     let input_vals = [0.0; 100];  // <- Containing outliers
//!     let mut filtered_vals = [0.0; 100];
//!     for (i, val) in input_vals.iter().enumerate() {
//!         filtered_vals[i] = filter.update(*val);
//!     }
//!     // filtered_vals <-- Outliers have been removed
//! }
//! ```

#![no_std]

use core::mem::MaybeUninit;
use num_traits::{cast, float::FloatCore};


/// Window of Hampel filter
pub struct Window<T: FloatCore, const WINDOW_SIZE: usize> {
    window: [T; WINDOW_SIZE],
    working_array: [T; WINDOW_SIZE],
    oldest: usize,
    coef: T,  // 閾値判定に使う係数
}

impl<T: FloatCore, const WINDOW_SIZE: usize> Window<T, WINDOW_SIZE> {
    /// * `init_val`: Initialization value of window.
    /// * `n_sigma`: Threshold for determining an outlier.
    /// 
    /// If the window's input value exceeds the `window's standard deviation` * `n_sigma`, 
    /// it is determined to be an outlier.
    pub fn new(init_val: T, n_sigma: T) -> Self {
        assert!(WINDOW_SIZE >= 3, "WINDOW_SIZE must be at least 3");

        Self {
            window: [init_val; WINDOW_SIZE],
            working_array: unsafe { MaybeUninit::uninit().assume_init() },
            oldest: 0,
            coef: cast::<f32, T>(1.4826).unwrap() * n_sigma,  // 1.4826は正規分布にするための係数
        }
    }

    /// Update element in window.
    /// 
    /// If `x` is above the threshold, return the `median of window`.
    /// If `x` is below the threshold, return `x` as is.
    pub fn update(&mut self, x: T) -> T {
        // range of `oldest`: [0, WINDOW_SIZE)
        unsafe {*self.window.get_unchecked_mut(self.oldest) = x};
        self.oldest = (self.oldest + 1) % WINDOW_SIZE;

        self.working_array = self.window;
        // ウィンドウの中央値を計算
        self.sort_working_array();
        let w0 = self.working_array[WINDOW_SIZE / 2];
        // ウィンドウの各値に対して，中央値との絶対差分を取る
        for w in self.working_array.iter_mut() {
            *w = (*w - w0).abs();
        }
        // 絶対差分を取ったので再度中央値を計算
        self.sort_working_array();
        let s0 = self.working_array[WINDOW_SIZE / 2];

        if (x - w0).abs() > self.coef * s0 {w0} else {x}
    }

    /// Insertion sort
    #[inline]
    fn sort_working_array(&mut self) {
        for i in 1..WINDOW_SIZE {
            let mut j = i;
            while j > 0 {
                let j_pre = j - 1;
                if unsafe{ self.working_array.get_unchecked(j_pre) > self.working_array.get_unchecked(j) } {
                    self.working_array.swap(j_pre, j);
                    j = j_pre;
                } else {
                    break;
                }
            }
        }
    }
}
