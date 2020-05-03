Carsier
========
Carsier is a build system brings experience of Cargo and Coursier to Scala.

Features
--------
1. Declare dependencies in toml file
    - Not turing complete
    - Use features to select (TODO)
    - Support both crates, scala and java packages
    ```toml
    [dependencies]
    # deps with org is by default a scala dependency
    breeze = { version = "*", org = "org.scalanlp" }
    # or specify it is a java dependency
    postgresql = { version = "*", org = "org.postgresql", java = true }
    # or a crates
    mycrate = "*"
    ```
2. Relative module system
    * implicit module name from file system
        * `a/b/c.scala` => `a.b.c`
        * `a/b/c/lib.scala` => `a.b.c`
        * `a/b.c.scala` => `a.b.c` (TODO)
    * import from relative
        * `%` => root, `%%` => current, `%^` => parent
        * `%:` => crates (TODO)
    * 2 ways to handle relative module
        * preprocess (text based, must starts_with "import %")
        * scala plugin (TODO)
    ````scala
    /// it would automatically convert to
    /// ```scala
    /// package crates.crate_name.path.to.file;
    /// import _root_.{crates => %:}; // TODO
    /// import %:.{crate_name => %};
    /// import %:.crate_name.path.to.{file => %%};
    /// import %:.crate_name.path.{to => %^};
    /// import %:.crate_name.{path => %^^};
    /// import %:.{crate_name => %^^^};
    /// ```
    package %%;
    ````
    * you could also specify package name by absolute/relative path
    ```scala
    /// in src/factory/users/extends/lib.scala where %% resolves to %.factory.extends
    /// would have package name to `crates.crate_name.factory.users.extra`
    package %^.extra;
    ```
3. target & features
    * default target `lib`, `bin`, `examples`, `tests`
        * `bin` expands to `bin_main`, detect `src/main.scala` by default
        * `examples` expands to [`example_name`, ...], for `name` of files in `examples` folder
        * `tests` expands to [`test_name`] for `name` in `test` folder
    * features with `name` or `path.to.dep/name` could be selected
    * there're special conflicting features `os` and `target`
        * `os = { conflict = ture, group = [ 'macos', 'unix', 'linux', 'windows' ] }`
        * `target = { conflict = true, group = [ 'x86_64', 'x86' ] }`
    * virtual features would be set iff all conditions in group met
        * `virtual = { virtual = true, group = [ 'a', 'b', 'c|d', '!(a|c)&d' ] }`
        * logic operators `!`, `|`, `&`
        * compare operators (only for version now) `<`, `<=`, `>`, `>=`, `=`, `!=`
    * files would be automacitally select by feature (TODO)
        * `filename-feature1-feature2.scala`
        * all file would be included iff all conditions in group met

Cli
------
* `carsier new demo && cd demo`
* `carsier build` or resolve
* `carsier run` # TODO
