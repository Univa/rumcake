# Contributing to rumcake

We appreciate your interest in contributing to rumcake!

This document contains information about how you can help improve rumcake, including
different ways to contribute, and general guidelines for submitting contributions.

<!--toc:start-->
  - [Code Contributions](#code-contributions)
    - [Doc Comments / API Reference](#doc-comments-api-reference)
    - [Making a pull request](#making-a-pull-request)
  - [New Features](#new-features)
  - [Testing & Reporting Issues](#testing-reporting-issues)
  - [Writing User Docs](#writing-user-docs)
    - [How to Develop User Docs](#how-to-develop-user-docs)
    - [Doc Guidelines](#doc-guidelines)
<!--toc:end-->

## Code Contributions

At it's core, `rumcake` is a Rust library.

At the minimum, developers that would like to make code contributions to `rumcake` should be familiar with:

- The [embassy framework](https://github.com/embassy-rs/embassy)
- Writing async Rust code

To see what tasks you can work on, consider looking at the [GitHub issues tab](https://github.com/Univa/rumcake/issues).

If you are an external contributor, please make a fork of the repository to commit your changes to.

### Doc Comments / API Reference

Please try to document any public APIs in the source code when possible.

The API reference is generated using a [custom script](./docs/scripts/gen_api_docs.js) that calls `cargo doc`
with different feature flags, and modifies the HTML with a navbar allowing you to see different versions of the API reference.

To see your API docs, you should:

- Run `yarn build` in the `/docs` folder to build the API reference (which uses the custom script mentioned above).
- Check the output using `yarn preview`, and navigating to the API reference.
- Commit changes once satisfied.

### Making a pull request

Once you are ready, feel free to make a pull request that points to the `main` branch of the main `rumcake` repository.

To make reviewing pull requests easier for contributors:

- Summarize the scope of the pull request in the title
  - You may also reference any related issue numbers in the title
- Follow the pull request template to help explain:
  - The type of changes made
  - Related GitHub issues that the pull request would resolve (if applicable)
  - Quick description of the changes made
  - Reproducible instructions to test the changes made
- Ensure that you have completed everything in the check-list provided in the pull request template

If you are unsure about any of the changes made in the pull request, feel free to indicate that in the description, and a reviewer should be able to guide you.

Once the pull request is approved, it will be merged in using a squash commit.

## New Features

If there is a feature that either you want to see in `rumcake`, or you want to implement yourself,
please consider opening a feature request in the [issues tab](https://github.com/Univa/rumcake/issues)
using the corresponding template. Please add the `enhancement` label to your issue.

This gives contributors the chance to discuss its feasibility, scope and design before it is built.

Please keep in mind that some features may be out of scope for `rumcake`, and may be rejected.

## Testing & Reporting Issues

Users can also contribute to `rumcake` by testing it on real hardware.
Any issues can be reported as a [GitHub issue](https://github.com/Univa/rumcake/issues),
using the "Bug Report" template.

Before you submit your issue, please make sure that there is no duplicate issue
reported in the [issues tab](https://github.com/Univa/rumcake/issues).

**Please avoid submitting any issues related to flashing firmware in the main `rumcake` repository.
If you have an issue flashing with one of the [templates](https://github.com/Univa/rumcake-templates), consider submitting an issue to the [`rumcake-templates`](https://github.com/Univa/rumcake-templates)
repository instead.**

When submitting an issue on GitHub:

- Summarize the issue in the title
- Provide a [minimum reproducible example](https://stackoverflow.com/help/minimal-reproducible-example) of your issue.
  - A link to a Cargo workspace that can be compiled would be best
- Include:
  - Information about the hardware you are testing on (MCU, peripheral devices like LED drivers, etc.)
  - Version of `rumcake` in use (in your `Cargo.toml`)
  - Steps to reproduce the issue
  - Videos, screenshots or logs of the issue
  - Observed behaviour vs expected behaviour
  - Any additional context that you feel is necessary

## Writing User Docs

The [user docs](https://univa.github.io/rumcake/) contains information
pertaining to the usage of the `rumcake` library. This includes documents with example implementations
of certain keyboard features, `rumcake` project setup, usage of the `rumcake` API, and anything else
that a developer using the `rumcake` library may want to know.

The user docs have the following goals:

- Must contain information that is accurate to the latest commit of the `rumcake` repository
- Must be easy to follow and straight to the point.

Please keep these goals in mind when developing documentation.

### How to Develop User Docs

Technical information about the user docs site can be found in [`/docs/README.md`](./docs/README.md).

The general process involves:

- Running `yarn dev` to start a development server.
- Editing documentation in the [`src`](./docs/src) folder.
- Previewing the changes made at `localhost:4321/rumcake/` in your browser.
- Committing changes once satisfied.
- [Making a pull request](#making-a-pull-request).

If you would like to also modify the API reference, make sure to see the instructions in the [API reference section](#doc-comments-api-reference).

### Doc Guidelines

The user docs aim to provide short and easy-to-follow guides on how to set up rumcake, and implement certain keyboard features using rumcake.

To achieve this, keep the following guidelines in mind:

- Avoid unnecessary technical explanations about the internals of `rumcake`.
- Provide **_reproducible_** code examples.
  - Use [Expressive Code features](https://starlight.astro.build/guides/authoring-content/#expressive-code-features), to highlight important parts of the code, or express changes in the code.
    - If you are writing a guide that involves chunking a large code example into multiple code blocks, use the `diff` syntax to show how the code evolves. Avoid chunking the code example into disjoint snippets of code when possible. See [the backlight docs](https://univa.github.io/rumcake/features/feature-backlight/#required-code) for an example.
  - Keep naming conventions consistent between code examples, both within the guide, and with other guides.
  - Only link to the API reference if you think that the user may benefit from that information. For example, listing all the possible enum members that they can use for a certain feature.
- Use a note (blue box) if you think your guide has implications on the implementation or behaviour of other `rumcake` features OR if you think another `rumcake` feature can have an implication on how the guide is followed.
- Use a tip (purple box) if you think that the user can benefit from an extra implementation detail.
- Use a caution (yellow box) if you think that the guide can potentially lead to behaviour that the user may not expect.

Once you have written your documentation, try following it step-by-step yourself to ensure that
it has all the necessary information with the appropriate sections.
