//! # Clock（時刻プロバイダ）
//!
//! ユースケース層での `Utc::now()` 直接呼び出しを置き換え、
//! テストで固定時刻を注入可能にするための抽象化。

use chrono::{DateTime, Utc};

/// 現在時刻を提供するトレイト
pub trait Clock: Send + Sync {
   fn now(&self) -> DateTime<Utc>;
}

/// 実際のシステム時刻を返す実装
pub struct SystemClock;

impl Clock for SystemClock {
   fn now(&self) -> DateTime<Utc> {
      Utc::now()
   }
}

/// 固定時刻を返すテスト用実装
pub struct FixedClock {
   now: DateTime<Utc>,
}

impl FixedClock {
   pub fn new(now: DateTime<Utc>) -> Self {
      Self { now }
   }
}

impl Clock for FixedClock {
   fn now(&self) -> DateTime<Utc> {
      self.now
   }
}

#[cfg(test)]
mod tests {
   use super::*;

   #[test]
   fn test_system_clock_は現在時刻を返す() {
      let clock = SystemClock;
      let before = Utc::now();
      let result = clock.now();
      let after = Utc::now();

      assert!(result >= before);
      assert!(result <= after);
   }

   #[test]
   fn test_fixed_clock_はコンストラクタで渡した時刻を返す() {
      let fixed_time = Utc::now();
      let clock = FixedClock::new(fixed_time);

      assert_eq!(clock.now(), fixed_time);
   }

   #[test]
   fn test_fixed_clock_は複数回呼んでも同じ時刻を返す() {
      let fixed_time = Utc::now();
      let clock = FixedClock::new(fixed_time);

      let first = clock.now();
      let second = clock.now();
      let third = clock.now();

      assert_eq!(first, fixed_time);
      assert_eq!(second, fixed_time);
      assert_eq!(third, fixed_time);
   }
}
