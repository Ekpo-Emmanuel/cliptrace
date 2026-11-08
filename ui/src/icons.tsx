import type { LucideIcon, LucideProps } from "lucide-react";
import {
  Check,
  CircleCheck,
  Code,
  Copy,
  Eye,
  EyeOff,
  FileText,
  Gauge,
  Keyboard,
  Link,
  Lock,
  Maximize2,
  Minimize2,
  Plus,
  Pin,
  Search,
  Settings,
  Sun,
  Moon,
  Shield,
  Sparkles,
  Tag,
  Trash2,
  X,
} from "lucide-react";

/** DESIGN.md has no icon token. Stitch uses 14–18px outlined glyphs. */
export const ICON_STROKE = 1.75;

function icon(Icon: LucideIcon) {
  return function CliptraceIcon({
    className = "h-3.5 w-3.5",
    strokeWidth = ICON_STROKE,
    ...props
  }: LucideProps) {
    return (
      <Icon
        aria-hidden
        strokeWidth={strokeWidth}
        className={className}
        {...props}
      />
    );
  };
}

export const IconSparkles = icon(Sparkles);
export const IconPin = icon(Pin);
export const IconTag = icon(Tag);
export const IconLock = icon(Lock);
export const IconClose = icon(X);
export const IconShield = icon(Shield);
export const IconFileText = icon(FileText);
export const IconLink = icon(Link);
export const IconCode = icon(Code);
export const IconCheck = icon(Check);
export const IconTrash = icon(Trash2);
export const IconCopy = icon(Copy);
export const IconPlus = icon(Plus);
export const IconEye = icon(Eye);
export const IconEyeOff = icon(EyeOff);
export const IconSearch = icon(Search);
export const IconKeyboard = icon(Keyboard);
export const IconCircleCheck = icon(CircleCheck);
export const IconGauge = icon(Gauge);

export const IconExpand = icon(Maximize2);
export const IconCollapse = icon(Minimize2);
export const IconSettings = icon(Settings);
export const IconSun = icon(Sun);
export const IconMoon = icon(Moon);
