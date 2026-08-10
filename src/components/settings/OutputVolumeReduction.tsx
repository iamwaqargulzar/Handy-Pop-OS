import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../hooks/useSettings";
import { Slider } from "../ui/Slider";

export const OutputVolumeReduction: React.FC<{ disabled?: boolean }> = ({
  disabled = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();
  const percentage = getSetting("output_volume_reduction_percent") ?? 0;

  return (
    <Slider
      value={percentage}
      onChange={(value) =>
        updateSetting("output_volume_reduction_percent", value)
      }
      min={0}
      max={90}
      step={5}
      label={t("settings.debug.lowerVolumeWhileRecording.label")}
      description={t(
        "settings.debug.lowerVolumeWhileRecording.description",
      )}
      descriptionMode="tooltip"
      grouped
      formatValue={(value) => `${Math.round(value)}%`}
      disabled={disabled}
    />
  );
};
