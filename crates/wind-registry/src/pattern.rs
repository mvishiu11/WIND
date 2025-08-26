use glob::{Pattern, PatternError};

/// Pattern matcher for service names supporting glob patterns
/// Examples: "SENSOR/*/TEMP", "DET/CHAMBER_*/STATUS"
#[derive(Debug, Clone)]
pub struct ServicePattern {
    pattern: Pattern,
    raw: String,
}

impl ServicePattern {
    pub fn new(pattern: &str) -> Result<Self, PatternError> {
        Ok(Self {
            pattern: Pattern::new(pattern)?,
            raw: pattern.to_string(),
        })
    }
    
    pub fn matches(&self, service_name: &str) -> bool {
        self.pattern.matches(service_name)
    }
    
    pub fn pattern_str(&self) -> &str {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pattern_matching() {
        let pattern = ServicePattern::new("SENSOR/*/TEMP").unwrap();
        
        assert!(pattern.matches("SENSOR/ROOM1/TEMP"));
        assert!(pattern.matches("SENSOR/ROOM2/TEMP"));
        assert!(!pattern.matches("SENSOR/ROOM1/HUMIDITY"));
        assert!(!pattern.matches("DETECTOR/ROOM1/TEMP"));
    }
    
    #[test] 
    fn test_complex_patterns() {
        let pattern = ServicePattern::new("DET/CHAMBER_*/STATUS").unwrap();
        
        assert!(pattern.matches("DET/CHAMBER_1/STATUS"));
        assert!(pattern.matches("DET/CHAMBER_A1/STATUS"));
        assert!(!pattern.matches("DET/ROOM_1/STATUS"));
    }
}
