// API doc generation is inspired by they way embassy-rs generates their docs: https://github.com/embassy-rs/docserver

import process from "child_process";
import path from "path";
import fs from "fs";
import url from "url";
import handlebars from "handlebars";
import toml from "toml";

function main() {
  const manifest = toml.parse(
    fs
      .readFileSync(
        path.join(
          path.dirname(url.fileURLToPath(import.meta.url)),
          path.join("..", "..", "rumcake", "Cargo.toml").toString(),
        ),
      )
      .toString(),
  );

  const redirect_template = handlebars.compile(
    fs
      .readFileSync(
        path.join(
          path.dirname(url.fileURLToPath(import.meta.url)),
          "redirect.html.hbs",
        ),
      )
      .toString(),
  );
  const nav_template = handlebars.compile(
    fs
      .readFileSync(
        path.join(
          path.dirname(url.fileURLToPath(import.meta.url)),
          "nav.html.hbs",
        ),
      )
      .toString(),
  );
  const head = handlebars.compile(
    fs
      .readFileSync(
        path.join(
          path.dirname(url.fileURLToPath(import.meta.url)),
          "head.html.hbs",
        ),
      )
      .toString(),
  )({});

  const end_of_head = /<\/head>/;
  const start_of_body = /<body class="([^\"]*)">/;
  const end_of_body = /<\/body>/;

  const update_html_in_folder = (folder, navbar) => {
    for (const file of fs
      .readdirSync(folder, { withFileTypes: true })
      .filter(
        (file) =>
          file.isDirectory() || (file.isFile() && file.name.endsWith(".html")),
      )) {
      // Update html in the folder
      if (file.isDirectory()) {
        update_html_in_folder(path.join(folder, file.name), navbar);
        continue;
      }

      // Otherwise, we have an HTML file, update the contents
      let data = fs.readFileSync(path.join(folder, file.name), "utf8");
      data = data.replace(end_of_head, `${head}</head>`);
      data = data.replace(start_of_body, `<body>${navbar}<div class="$1">`);
      data = data.replace(end_of_body, "</div></body>");
      fs.writeFileSync(path.join(folder, file.name), data);
    }
  };

  const delete_lock_file = (folder) => {
    for (const file of fs
      .readdirSync(folder, { withFileTypes: true })
      .filter(
        (file) =>
          file.isDirectory() || (file.isFile() && file.name === ".lock"),
      )) {
      // Update html in the folder
      if (file.isDirectory()) {
        delete_lock_file(path.join(folder, file.name));
        continue;
      }

      // Otherwise, we have a lock file. Delete it.
      fs.rmSync(path.join(folder, file.name));
    }
  };

  let error = false;

  // generate docs
  for (const target of manifest.package.metadata.rumcake_docs.flavours) {
    const output_dir = path.join(".", "temp", target.feature);

    const rustdoc = process.spawnSync(
      "cargo",
      [
        "doc",
        "--no-deps",
        "--release",
        `--features=${
          target.feature
        },${manifest.package.metadata.rumcake_docs.features.join(
          ",",
        )},${target.extra_features.join(",")}`,
        "--target",
        target.triple, // build target triple
        "--target-dir",
        output_dir.toString(),
        "--manifest-path",
        path.join("..", "rumcake", "Cargo.toml").toString(),
        "--quiet",
      ],
      {
        stdio: "inherit",
      },
    );

    if (rustdoc.status) {
      error = true;
      console.error(`${target.feature} API docs failed to build.`);
      continue;
    }

    console.log(`${target.feature} API docs built.`);

    // Generate the navbar
    const navbar = nav_template({
      current_target: target.feature,
      targets: manifest.package.metadata.rumcake_docs.flavours,
    });

    // Update the HTML
    const doc_folder = path.join(output_dir, target.triple, "doc");
    update_html_in_folder(doc_folder, navbar);

    // Add redirect from root of the target's docs to the rumcake folder containing the docs
    fs.writeFileSync(
      path.join(doc_folder, "index.html"),
      redirect_template({ url: "rumcake" }),
    );

    // Remove the .lock file in the docs (GitHub doesn't like it because it doesn't have the right permissions)
    delete_lock_file(doc_folder);

    // Copy the results to ./dist
    fs.cpSync(doc_folder, path.join(".", "dist", "api", target.feature), {
      recursive: true,
    });
  }

  // Remove temp folder
  fs.rmSync(path.join(".", "temp"), { recursive: true });

  if (error) {
    throw new Error("Some docs failed to build.");
  }

  // Add an index.html to the /rumcake/api route to redirect to the docs for the first target
  fs.writeFileSync(
    path.join(".", "dist", "api", "index.html"),
    redirect_template({
      url: `${manifest.package.metadata.rumcake_docs.flavours[0].feature}/rumcake`,
    }),
  );
}

main();
