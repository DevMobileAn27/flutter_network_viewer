import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../models/capture.dart';
import '../services/form_body.dart';
import 'components.dart';

String bodyByteSize(int bytes) {
  if (bytes < 1024) return '$bytes B';
  final divisor = bytes < 1024 * 1024 ? 1024 : 1024 * 1024;
  final unit = divisor == 1024 ? 'KB' : 'MB';
  final amount = (bytes / divisor)
      .toStringAsFixed(1)
      .replaceFirst(RegExp(r'\.0$'), '');
  return '$amount $unit';
}

class BodyCaptureNotice extends StatelessWidget {
  const BodyCaptureNotice({super.key, required this.data});
  final Json data;
  @override
  Widget build(BuildContext context) {
    final limit = (data['bodyCaptureLimit'] as num?)?.toInt() ?? 0;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 12),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Icon(Icons.info_outline, color: context.palette.info, size: 18),
          const SizedBox(width: 8),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  context.l10n.bodyNotCaptured,
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                const SizedBox(height: 4),
                Text(
                  data['bodyOmitted'] == 'profile-size-limit'
                      ? context.l10n.bodyTooLarge(bodyByteSize(limit))
                      : context.l10n.bodyReadFailed,
                  style: TextStyle(color: context.palette.muted),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class FormBodyView extends StatefulWidget {
  const FormBodyView({super.key, required this.form, required this.raw});
  final FormBody form;
  final String raw;
  @override
  State<FormBodyView> createState() => _FormBodyViewState();
}

class _FormBodyViewState extends State<FormBodyView> {
  bool _raw = false;
  @override
  void didUpdateWidget(FormBodyView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.form != widget.form) _raw = false;
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    builder: (context, constraints) {
      final height = constraints.minHeight > 0 ? constraints.minHeight : 368.0;
      return SizedBox(
        height: height,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                Text(
                  context.l10n.formData,
                  style: TextStyle(color: context.palette.muted, fontSize: 12),
                ),
                const SizedBox(width: 8),
                Text(
                  context.l10n.itemCount(widget.form.parts.length),
                  style: TextStyle(color: context.palette.muted, fontSize: 11),
                ),
                const Spacer(),
                TextButton.icon(
                  key: const ValueKey('form-body-toggle'),
                  onPressed: () => setState(() => _raw = !_raw),
                  icon: Icon(_raw ? Icons.list_alt : Icons.code, size: 16),
                  label: Text(_raw ? context.l10n.formData : context.l10n.raw),
                ),
              ],
            ),
            if (widget.form.incomplete && !_raw)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: Text(
                  context.l10n.formIncomplete,
                  style: TextStyle(color: context.palette.warning),
                ),
              ),
            Expanded(
              child: _raw
                  ? SingleChildScrollView(
                      key: const ValueKey('form-body-raw'),
                      child: SelectableText(widget.raw, style: mono),
                    )
                  : Container(
                      key: const ValueKey('form-body-fields'),
                      decoration: BoxDecoration(
                        border: Border.all(color: context.palette.line),
                        borderRadius: BorderRadius.circular(6),
                      ),
                      clipBehavior: Clip.antiAlias,
                      child: LayoutBuilder(
                        builder: (context, size) {
                          final nameWidth = (size.maxWidth * .3).clamp(
                            70.0,
                            180.0,
                          );
                          return Column(
                            children: [
                              ColoredBox(
                                color: context.palette.input,
                                child: Padding(
                                  padding: const EdgeInsets.symmetric(
                                    horizontal: 10,
                                    vertical: 6,
                                  ),
                                  child: Row(
                                    children: [
                                      SizedBox(
                                        width: nameWidth,
                                        child: Text(context.l10n.formField),
                                      ),
                                      Expanded(
                                        child: Text(context.l10n.formValue),
                                      ),
                                    ],
                                  ),
                                ),
                              ),
                              Expanded(
                                child: ListView.separated(
                                  itemCount: widget.form.parts.length,
                                  separatorBuilder: (_, _) => Divider(
                                    height: 1,
                                    color: context.palette.line,
                                  ),
                                  itemBuilder: (context, index) {
                                    final part = widget.form.parts[index];
                                    return Padding(
                                      padding: const EdgeInsets.symmetric(
                                        horizontal: 10,
                                        vertical: 7,
                                      ),
                                      child: Row(
                                        crossAxisAlignment:
                                            CrossAxisAlignment.start,
                                        children: [
                                          SizedBox(
                                            width: nameWidth,
                                            child: Padding(
                                              padding: const EdgeInsets.only(
                                                right: 8,
                                              ),
                                              child: SelectableText(
                                                part.name,
                                                style: mono.copyWith(
                                                  color: context.palette.info,
                                                ),
                                              ),
                                            ),
                                          ),
                                          Expanded(
                                            child: !part.isFile
                                                ? SelectableText(
                                                    part.value ?? '',
                                                    style: mono,
                                                  )
                                                : Column(
                                                    crossAxisAlignment:
                                                        CrossAxisAlignment
                                                            .start,
                                                    children: [
                                                      Row(
                                                        crossAxisAlignment:
                                                            CrossAxisAlignment
                                                                .start,
                                                        children: [
                                                          Icon(
                                                            Icons
                                                                .insert_drive_file_outlined,
                                                            size: 16,
                                                            color: context
                                                                .palette
                                                                .muted,
                                                          ),
                                                          const SizedBox(
                                                            width: 6,
                                                          ),
                                                          Expanded(
                                                            child: SelectableText(
                                                              part
                                                                      .filename!
                                                                      .isEmpty
                                                                  ? context
                                                                        .l10n
                                                                        .formFile
                                                                  : part.filename!,
                                                              style: mono,
                                                            ),
                                                          ),
                                                        ],
                                                      ),
                                                      const SizedBox(height: 3),
                                                      SelectableText(
                                                        '${part.contentType ?? 'application/octet-stream'} · ${bodyByteSize(part.byteLength)}',
                                                        style: mono.copyWith(
                                                          fontSize: 11,
                                                          color: context
                                                              .palette
                                                              .muted,
                                                        ),
                                                      ),
                                                    ],
                                                  ),
                                          ),
                                        ],
                                      ),
                                    );
                                  },
                                ),
                              ),
                            ],
                          );
                        },
                      ),
                    ),
            ),
          ],
        ),
      );
    },
  );
}
