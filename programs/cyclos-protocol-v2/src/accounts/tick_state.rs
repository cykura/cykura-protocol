use anchor_lang::prelude::*;

#[account]  
pub struct TickState {
    pub bump: u8,
    pub liquidityNet: f64, 
    pub liquidityGross: f64, 
    pub feeGrowthOutside0: f64, 
    pub feeGrowthOutside1: f64, 
}