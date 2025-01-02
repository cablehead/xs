// index.ts
// based on: https://github.com/FujoWebDev/fujocoded-plugins/blob/main/expressive-code-output/index.ts

import { definePlugin, AttachedPluginData } from "@expressive-code/core";
import { h } from "@expressive-code/core/hast";
var outputData = new AttachedPluginData(() => ({ output: [] }));
function pluginCodeOutput() {
  return definePlugin({
    name: "Code output",
    baseStyles: ` 
    .expressive-code .frame pre.output {
      display: block;
      border: var(--ec-brdWd) solid var(--ec-brdCol);
      border-top: var(--ec-brdWd) dashed var(--ec-brdCol);
      padding: var(--ec-codePadBlk) 0;
      padding-inline-start: var(--ec-codePadInl);
    }
      `,
    hooks: {
      preprocessCode: (context) => {
        if (!context.codeBlock.meta.includes("withOutput")) return;
        const blockData = outputData.getOrCreateFor(context.codeBlock);
        const outputStart = context.codeBlock.getLines().findIndex((line) => !line.text.startsWith("> "));
        context.codeBlock.getLines(0, outputStart == -1 ? void 0 : outputStart).forEach((line) => {
          line.editText(0, 2, "");
        });
        if (outputStart === -1) return;
        context.codeBlock.getLines(outputStart).forEach((line) => {
          blockData.output.push(line.text);
        });
        for (let i = context.codeBlock.getLines().length; i > outputStart; i--) {
          context.codeBlock.deleteLine(i - 1);
        }
      },
      postprocessRenderedBlock: async (context) => {
        if (!context.codeBlock.meta.includes("withOutput")) return;
        const blockData = outputData.getOrCreateFor(context.codeBlock);
        if (!blockData.output.length) return;
        const lastPre = context.renderData.blockAst.children.findLastIndex(
          (child) => child.type === "element" && child.tagName === "pre"
        );
        if (lastPre === -1) return;
        const currentChildren = context.renderData.blockAst.children;
        const newChildren = [
          ...currentChildren.slice(0, lastPre + 1),
          h(
            "pre.output",
            blockData.output.map((line) => h("div", line))
          ),
          ...currentChildren.slice(lastPre + 1)
        ];
        context.renderData.blockAst.children = newChildren;
      }
    }
  });
}
export {
  pluginCodeOutput
};
