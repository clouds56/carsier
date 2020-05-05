package %%

import scala.tools.nsc
import nsc.Global
import nsc.Phase
import nsc.plugins.Plugin
import nsc.plugins.PluginComponent
import java.nio.file.{Path, Paths}
import Implicits._

object Implicits {
  implicit class StringWithTrim(val s: String) {
    def trimEndsMatches(pat: String): String = {
      val lastIndex = s.lastIndexOf(pat)
      if (lastIndex + pat.length == s.length) {
        s.dropRight(pat.length)
      } else {
        s
      }
    }
  }
}

class ModulePlugin(val global: Global) extends Plugin {
  import global._

  val name = "moduler"
  val description = "transform module"
  val components = List[PluginComponent](Component)

  private object Component extends PluginComponent {
    val global: ModulePlugin.this.global.type = ModulePlugin.this.global
    val runsAfter = List[String]("parser")
    override val runsBefore = List[String]("namer")
    val phaseName = ModulePlugin.this.name
    def newPhase(_prev: Phase) = new PackagePhase(_prev)
    class ModulerTransformer(basePackage: List[TermName], base: List[TermName]) extends Transformer {
      var level = 0
      var current: List[TermName] = base

      abstract class Prefix {

      }
      object Prefix {
        def from(term: TermName, pos: Position): Prefix = {
          val s = term.toString.replace("$percent", "%").replace("$up", "^").replace("$colon", ":")
          if (s == "%") { Absolute }
          else if (s == "%%") { Relative(0) }
          else if (s.matches("%\\^+")) { Relative(s.length - 1) }
          else if (s.startsWith("%")) {
            global.reporter.error(pos, "unknown head");
            Absolute
          } else {
            Other(term)
          }
        }
        case class Other(term: TermName) extends Prefix;
        case class Relative(n: Int) extends Prefix;
        case object Absolute extends Prefix;
      }

      override def transform(tree: Tree): Tree = {
        tree match {
          case PackageDef(select, content) => {
            // TODO: check empty
            val oldCurrent = this.current
            this.current = this.transformSelect(this.current, select)
            this.level += 1
            val absoluteSelect = this.asRefTree(this.current ++ basePackage.reverse)
            println(f"transform: $select => $absoluteSelect")
            val tree = PackageDef(select, super.transformTrees(content))
            this.level -= 1
            this.current = oldCurrent
            tree
          }
          case _ => tree
        }
      }

      def asRefTree(terms: List[TermName]): RefTree = {
        terms match {
          case List(term) => Ident(term)
          case term :: tail => Select(this.asRefTree(tail), term)
        }
      }

      def transformSelect(old: List[TermName], select: RefTree): List[TermName] = {
        select match {
          case Ident(term: TermName) => {
            Prefix.from(term, select.pos) match {
              case Prefix.Absolute => List()
              case Prefix.Relative(n) => {
                if (n > old.length) {
                  global.reporter.error(select.pos, "relative cross root");
                }
                old.dropRight(n)
              }
              case Prefix.Other(term) => term :: old
            }
          }
          case Select(select: RefTree, term: TermName) => term :: this.transformSelect(old, select)
        }
      }
    }
    class PackagePhase(prev: Phase) extends StdPhase(prev) {
      override def name = Component.this.phaseName
      val basePath = Paths.get("src") // TODO: config via options
      val basePackage = List(TermName("crates"))
      def apply(unit: CompilationUnit): Unit = {
        val path = Paths.get(unit.source.file.path).normalize
        if (path.startsWith(this.basePath)) {
          val module = this.pathToModule(basePath.relativize(path))
          println(f"processing unit: $path => $module")
          new ModulerTransformer(this.basePackage, module.toList).transformUnit(unit)
        } else {
          global.reporter.error(unit.body.pos, "file out of source tree")
        }
      }
      def pathToModule(path: Path): Seq[TermName] = {
        (0 until path.getNameCount)
          .map(path.getName(_).toString.trimEndsMatches(".scala"))
          .filter(_ != "lib").map(TermName(_))
      }
    }
  }
}

package test1 {
  package c {}
}
package test2 {}
