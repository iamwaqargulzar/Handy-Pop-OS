import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { Cpu } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Dialog } from "@/components/ui/Dialog";
import type { ModelStateEvent } from "@/lib/types/events";
import { useModelStore } from "@/stores/modelStore";

interface LoadingNpuModel {
  id: string;
  name: string;
}

export const NpuModelLoadingOverlay = () => {
  const { t } = useTranslation();
  const [loadingModel, setLoadingModel] = useState<LoadingNpuModel | null>(
    null,
  );

  useEffect(() => {
    const unlisten = listen<ModelStateEvent>("model-state-changed", (event) => {
      const {
        event_type: eventType,
        model_id: modelId,
        model_name: modelName,
      } = event.payload;

      if (eventType === "loading_started" && modelId) {
        const model = useModelStore
          .getState()
          .models.find((candidate) => candidate.id === modelId);

        if (model?.engine_type === "OpenVinoNpu") {
          setLoadingModel({ id: modelId, name: modelName || model.name });
        }
        return;
      }

      if (
        eventType === "loading_completed" ||
        eventType === "loading_failed" ||
        eventType === "unloaded"
      ) {
        setLoadingModel((current) =>
          !modelId || current?.id === modelId ? null : current,
        );
      }
    });

    return () => {
      unlisten.then((removeListener) => removeListener());
    };
  }, []);

  return (
    <Dialog
      open={loadingModel !== null}
      title={t("modelSelector.npuLoading.title")}
      description={t("modelSelector.npuLoading.description", {
        modelName: loadingModel?.name,
      })}
      onOpenChange={() => undefined}
      closeLabel={t("modelSelector.npuLoading.closeLabel")}
      dismissible={false}
      closeOnBackdrop={false}
      showCloseButton={false}
      contentFades={false}
      className="max-w-md"
    >
      <div className="flex gap-3 rounded-lg border border-logo-primary/20 bg-logo-primary/5 p-3">
        <Cpu
          className="mt-0.5 h-5 w-5 shrink-0 text-logo-primary"
          aria-hidden="true"
        />
        <p className="text-sm leading-5 text-mid-gray">
          {t("modelSelector.npuLoading.explanation")}
        </p>
      </div>

      <div className="mt-4" aria-live="polite">
        <div
          className="h-2 overflow-hidden rounded-full bg-mid-gray/15"
          role="progressbar"
          aria-label={t("modelSelector.npuLoading.progressLabel")}
        >
          <div className="npu-loading-progress h-full rounded-full bg-logo-primary" />
        </div>
        <p className="mt-2 text-center text-xs text-mid-gray">
          {t("modelSelector.npuLoading.waitMessage")}
        </p>
      </div>
    </Dialog>
  );
};
