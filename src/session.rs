//! Authoritative lifecycle. Late and cancelled jobs cannot become deliverable results.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Ready,
    Listening,
    Processing,
    Held,
    Error,
}

#[derive(Debug)]
pub struct Session {
    pub id: u64,
    pub phase: Phase,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            id: 0,
            phase: Phase::Ready,
        }
    }
}
impl Session {
    pub fn start(&mut self) -> Result<u64, &'static str> {
        if matches!(self.phase, Phase::Listening | Phase::Processing) {
            return Err("busy");
        }
        self.id = self.id.wrapping_add(1);
        self.phase = Phase::Listening;
        Ok(self.id)
    }
    pub fn stop(&mut self) -> Result<u64, &'static str> {
        if self.phase != Phase::Listening {
            return Err("not recording");
        }
        self.phase = Phase::Processing;
        Ok(self.id)
    }
    pub fn finish(&mut self, id: u64, has_text: bool) -> bool {
        if id != self.id || self.phase != Phase::Processing {
            return false;
        }
        self.phase = if has_text { Phase::Held } else { Phase::Ready };
        true
    }
    pub fn cancel(&mut self) {
        self.id = self.id.wrapping_add(1);
        self.phase = Phase::Ready;
    }
    pub fn fail(&mut self, id: u64) -> bool {
        if self.id != id {
            return false;
        }
        self.phase = Phase::Error;
        true
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_result_never_becomes_held() {
        let mut s = Session::default();
        let old = s.start().unwrap();
        s.stop().unwrap();
        s.cancel();
        s.start().unwrap();
        s.stop().unwrap();
        assert!(!s.finish(old, true));
        assert_eq!(s.phase, Phase::Processing);
    }
    #[test]
    fn duplicate_start_and_finish_rejected() {
        let mut s = Session::default();
        let id = s.start().unwrap();
        assert!(s.start().is_err());
        assert!(!s.finish(id, true));
        s.stop().unwrap();
        assert!(s.finish(id, true));
        assert!(!s.finish(id, true));
    }
    #[test]
    fn silence_returns_ready() {
        let mut s = Session::default();
        let id = s.start().unwrap();
        s.stop().unwrap();
        assert!(s.finish(id, false));
        assert_eq!(s.phase, Phase::Ready);
    }
}
