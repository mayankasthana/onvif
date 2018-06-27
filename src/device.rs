#[derive(Builder)]
#[builder(setter(into))]
pub struct OnvifDevice {
  xaddr: String,
  user: String,
  password: String,
}

impl OnvifDevice {
  pub fn xaddr(&self) -> &String {
    &self.xaddr
  }
  pub fn user(&self) -> &String {
    &self.user
  }
}
