package %%;

class A {
  val c = new %.c.C;
  val m = new %.M;
  val a1 = new t.A1;
  val text = "world";
}

package t {
  class A1 {
    // TODO: warn this
    val a = new A;
  }
}
