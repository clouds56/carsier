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
    * import from relative `%` => root, `%%` => current, `%^` => parent, `%:` => crates
    * 2 ways to handle relative module
    * preprocess (text based, must starts_with "import %")
    * scala plugin (TODO)
    ````scala
    /// it would automatically convert to
    /// ```scala
    /// package crates.crate_name.path.to.file;
    /// import crates.{crate_name => %};
    /// import crates.crate_name.path.to.{file => %%};
    /// import crates.crate_name.path.{to => %^};
    /// import crates.crate_name.{path => %^^};
    /// import crates.{crate_name => %^^^};
    /// ```
    package %%;
    /// TODO: paragraph about specify package name
    ````

Cli
------
* `carsier new demo && cd demo`
* `carsier build` or resolve
* `carsier run` # TODO
