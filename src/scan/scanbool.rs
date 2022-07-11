pub struct scanbool {
    pub scan: bool,
    pub power: u64 //预期算力
}

impl scanbool {
   pub fn new() -> scanbool{
        scanbool{
            scan: false,
            power: 10000
        }
    }
   pub fn update(&mut self,b: bool){
        self.scan = b
    }
   pub fn updatePowerEx(&mut self,power: u64) {
       self.power = power
   }
}