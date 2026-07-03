export function renderCheckList(root: HTMLElement, models: string[], selection: Set<string>, prefix: string, emptyText: string) {
  root.innerHTML = "";
  root.classList.toggle("empty", models.length === 0);
  if (models.length === 0) {
    root.textContent = emptyText;
    return;
  }
  for (const model of models) {
    const item = document.createElement("label");
    item.className = "model-check";
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = selection.has(model);
    item.classList.toggle("checked", checkbox.checked);
    checkbox.id = `${prefix}-${model}`;
    checkbox.addEventListener("change", () => {
      if (checkbox.checked) selection.add(model);
      else selection.delete(model);
      item.classList.toggle("checked", checkbox.checked);
    });
    const text = document.createElement("span");
    text.textContent = model;
    item.append(checkbox, text);
    root.append(item);
  }
}

export function setSelection(selection: Set<string>, models: string[], checked: boolean, render: () => void) {
  selection.clear();
  if (checked) {
    for (const model of models) selection.add(model);
  }
  render();
}

export function invertSelection(selection: Set<string>, models: string[], render: () => void) {
  for (const model of models) {
    if (selection.has(model)) selection.delete(model);
    else selection.add(model);
  }
  render();
}
