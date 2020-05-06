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

  var basePath = Paths.get("src") // default to "src" folder
  var registry = List(TermName("crates")) // note: reverse
  var packageName: TermName = termNames.EMPTY_PACKAGE_NAME
  var entryPoint = "main" // TODO

  def basePackage = packageName :: registry

  override def init(options: List[String], error: String => Unit): Boolean = {
    for (option <- options) {
      if (option.startsWith("src=")) {
        basePath = Paths.get(option.substring("src=".length))
      } else if (option.startsWith("registry=")) {
        registry = option.substring("registry=".length).split('.').toList.map(TermName(_))
      } else if (option.startsWith("name=")) {
        packageName = TermName(option.substring("name=".length))
      } else if (option.startsWith("entry-point=")) {
        entryPoint = option.substring("entry-point=".length)
      } else {
        error("Option not understood: "+option)
      }
    }
    true
  }

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
        def toTerm = TermName(this.toString.replace("%", "$percent").replace("^", "$up").replace(":", "$colon"))
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
        case class Other(term: TermName) extends Prefix {
          override def toString = term.toString
        }
        case class Relative(n: Int) extends Prefix {
          override def toString = n match {
            case 0 => "%%"
            case n => "%" + "^".repeat(n)
          }
        }
        case object Absolute extends Prefix {
          override def toString = "%"
        }
      }

      override def transform(tree: Tree): Tree = {
        tree match {
          case PackageDef(select, content) => {
            // TODO: check empty
            val oldCurrent = this.current
            this.current = this.transformSelect(this.current, select, this.level)
            this.level += 1
            val absoluteSelect = this.getRefTree(this.current ++ basePackage) // FIXME: check root
            val imports = this.genImport(this.current, 0)
            println(f"transform: $select => $absoluteSelect")
            val tree = PackageDef(if (level == 1) { absoluteSelect } else { select }, imports ++ super.transformTrees(content))
            this.level -= 1
            this.current = oldCurrent
            tree
          }
          case _ => tree
        }
      }

      def genImport(terms: List[TermName], depth: Int): List[Tree] = {
        val current = terms ++ basePackage
        val select = getRefTree(current.tail, root=true)
        val importStmt = Import(select, List(ImportSelector(current.head, -1, Prefix.Relative(depth).toTerm, -1)))
        println(f"asImport $importStmt")
        if (terms.isEmpty) {
          val select = getRefTree(basePackage.tail, root=true)
          val importStmt2 = Import(select, List(ImportSelector(basePackage.head, -1, Prefix.Absolute.toTerm, -1)))
          List(importStmt, importStmt2)
        } else {
          importStmt :: genImport(terms.tail, depth + 1)
        }
      }

      def getRefTree(terms: List[TermName], root: Boolean = false): RefTree = {
        terms match {
          case List(term) => if (root) {
            Select(Ident(termNames.ROOTPKG), term)
          } else { Ident(term) }
          case term :: tail => Select(getRefTree(tail, root), term)
        }
      }

      def transformSelect(old: List[TermName], select: RefTree, level: Int): List[TermName] = {
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
              case Prefix.Other(term) => {
                if (level == 0) { List(term) } else { term :: old }
              }
            }
          }
          case Select(select: RefTree, term: TermName) => term :: transformSelect(old, select, level)
        }
      }
    }
    class PackagePhase(prev: Phase) extends StdPhase(prev) {
      override def name = Component.this.phaseName
      def apply(unit: CompilationUnit): Unit = {
        val path = Paths.get(unit.source.file.path).normalize
        if (path.startsWith(basePath)) {
          val module = pathToModule(basePath.relativize(path)).toList
          println(f"processing unit: $path => $module")
          new ModulerTransformer(basePackage, module).transformUnit(unit)
        } else {
          global.reporter.error(unit.body.pos, "file out of source tree")
        }
      }
      def pathToModule(path: Path): Seq[TermName] = {
        val result = (0 until path.getNameCount)
          .map(path.getName(_).toString.trimEndsMatches(".scala"))
          .filter(_ != "lib").map(TermName(_))
        if (result == List(TermName(entryPoint))) {
          List()
        } else {
          result
        }
      }
    }
  }
}

package test1 {
  package c {}
}
package test2 {}
