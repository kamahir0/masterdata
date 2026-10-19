import { useState, type MouseEvent } from "react";
import { Select, type SelectProps } from "antd";
import type { BaseOptionType, DefaultOptionType } from "antd/es/select";

// A search field remains editable while typing. With no query, a second click
// on the same single-choice field acts like its arrow and closes the popup.
export function SearchSelect<
  Value = any,
  Option extends BaseOptionType | DefaultOptionType = DefaultOptionType,
>(props: SelectProps<Value, Option>) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const expanded = props.open ?? open;
  const changeOpen = (value: boolean) => {
    setOpen(value);
    if (!value) setQuery("");
    props.onOpenChange?.(value);
  };
  const mouse = {
    onMouseDownCapture(event: MouseEvent<HTMLDivElement>) {
      if (
        event.button !== 0 ||
        props.mode ||
        !expanded ||
        query ||
        !event.currentTarget.contains(event.target as Node) ||
        (event.target as HTMLElement).closest(
          ".ant-select-clear, .ant-select-dropdown",
        )
      )
        return;
      event.preventDefault();
      event.stopPropagation();
      changeOpen(false);
    },
  };
  return (
    <Select<Value, Option>
      showSearch
      optionFilterProp="label"
      {...props}
      {...mouse}
      open={expanded}
      onOpenChange={changeOpen}
      onSearch={(value) => {
        setQuery(value);
        if (value) changeOpen(true);
        props.onSearch?.(value);
      }}
    />
  );
}
