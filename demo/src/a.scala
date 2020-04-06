package %%;

class A {
  val c = new %.b.C;
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
