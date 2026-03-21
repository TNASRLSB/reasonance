export interface MenuItemDef {
  label?: string;
  shortcut?: string;
  action?: () => void;
  submenu?: MenuItemDef[];
  divider?: boolean;
}
