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
/// 
/// * `WINDOW_SIZE` >= 3
pub struct Window<T: FloatCore, const WINDOW_SIZE: usize> {
    window: [T; WINDOW_SIZE],
    working_array: [T; WINDOW_SIZE],
    oldest: usize,  // window内の最も古い要素のインデックス
    coef: T,  // 閾値判定に使う係数
}

// n_sigmaを大きくするほど判定が緩くなる（外れ値を見落としやすくなる）
impl<T: FloatCore, const WINDOW_SIZE: usize> Window<T, WINDOW_SIZE> {
    /// * `init_val`: Initialization value of window.
    /// * `n_sigma`: Threshold for determining an outlier.
    /// 
    /// If the window's input value exceeds the `window's standard deviation` * `n_sigma`, 
    /// it is determined to be an outlier.
    /// The larger n_sigma is, the harder it is to detect outliers.
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
    /// When `x` is determined to be an outlier, the median value of the window is usually returned.
    /// If the `extrapolation` feature is enabled, the linear extrapolated value is returned.
    /// 
    /// When `x` is judged not to be an outlier, `x` is returned as is.
    pub fn update(&mut self, x: T) -> T {
        // Range of `oldest`: [0, WINDOW_SIZE)
        unsafe {*self.window.get_unchecked_mut(self.oldest) = x};
        self.oldest = (self.oldest + 1) % WINDOW_SIZE;

        self.working_array = self.window;
        // ウィンドウの中央値を計算
        let w0 = self.get_median();
        // ウィンドウの各値に対して，中央値との絶対差分を取る
        for w in self.working_array.iter_mut() {
            *w = (*w - w0).abs();
        }
        // 絶対差分を取ったので再度中央値を計算
        let s0 = self.get_median();

        // 外れ値かどうか判定
        if (x - w0).abs() <= self.coef * s0 {
            x
        } else {
            #[cfg(feature = "extrapolation")]
            {
                // 線形外挿した値を返す
                self.extrapolation()
            }
            
            #[cfg(not(feature = "extrapolation"))]
            {
                // ウィンドウの中央値を返す
                w0
            }
        }
    }

    /// working_arrayの中央値を返す
    fn get_median(&mut self) -> T {
        // Insertion sort
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
        
        self.working_array[WINDOW_SIZE / 2]
    }

    /// 一番最後に追加されたデータ（外れ値）を無視して線形外挿する
    #[cfg(feature = "extrapolation")]
    fn extrapolation(&self) -> T {
        // x座標を(0, 1, 2, ...)と取った場合の平均値（等差数列の平均）
        let mu_x = cast::<usize, T>(WINDOW_SIZE - 2).unwrap() * cast::<f32, T>(0.5).unwrap();

        // windowの平均値（外れ値を除いた平均値なので WINDOW_SIZE-1 になっている）
        let mut mu_y = T::zero();
        for i in 0..(WINDOW_SIZE - 1) {
            mu_y = mu_y + self.window[(self.oldest + i) % WINDOW_SIZE];
        }
        mu_y = mu_y / cast::<usize, T>(WINDOW_SIZE - 1).unwrap();

        let mut numer = T::zero();
        let mut denom = T::zero();
        for i in 0..(WINDOW_SIZE - 1) {
            let dev_x = cast::<usize, T>(i).unwrap() - mu_x;
            let dev_y = self.window[(self.oldest + i) % WINDOW_SIZE] - mu_y;

            numer = numer + dev_x * dev_y;
            denom = denom + dev_x * dev_x;
        }

        // 最小二乗法で求めた傾きと切片
        let a = numer / denom;  // denom=0となることは無い
        let b = mu_y - a * mu_x;

        a * cast::<usize, T>(WINDOW_SIZE - 1).unwrap() + b
    }
}
