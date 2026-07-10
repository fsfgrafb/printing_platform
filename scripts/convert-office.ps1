param(
    [Parameter(Mandatory = $true)]
    [string]$InputPath,

    [Parameter(Mandatory = $true)]
    [string]$OutputPath
)

$ErrorActionPreference = "Stop"

$resolvedInputPath = (Resolve-Path -LiteralPath $InputPath).Path
$resolvedOutputPath = [System.IO.Path]::GetFullPath($OutputPath)
$extension = [System.IO.Path]::GetExtension($resolvedInputPath).ToLowerInvariant()
$outputDir = [System.IO.Path]::GetDirectoryName($resolvedOutputPath)

if (-not [System.IO.Directory]::Exists($outputDir)) {
    [System.IO.Directory]::CreateDirectory($outputDir) | Out-Null
}

switch ($extension) {
    ".doc" {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        try {
            $doc = $word.Documents.Open($resolvedInputPath, $false, $true)
            try {
                $doc.ExportAsFixedFormat($resolvedOutputPath, 17)
            } finally {
                $doc.Close($false)
            }
        } finally {
            $word.Quit()
        }
    }
    ".docx" {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        try {
            $doc = $word.Documents.Open($resolvedInputPath, $false, $true)
            try {
                $doc.ExportAsFixedFormat($resolvedOutputPath, 17)
            } finally {
                $doc.Close($false)
            }
        } finally {
            $word.Quit()
        }
    }
    ".xls" {
        $excel = New-Object -ComObject Excel.Application
        $excel.Visible = $false
        $excel.DisplayAlerts = $false
        try {
            $book = $excel.Workbooks.Open($resolvedInputPath, 3, $true)
            try {
                $book.ExportAsFixedFormat(0, $resolvedOutputPath)
            } finally {
                $book.Close($false)
            }
        } finally {
            $excel.Quit()
        }
    }
    ".xlsx" {
        $excel = New-Object -ComObject Excel.Application
        $excel.Visible = $false
        $excel.DisplayAlerts = $false
        try {
            $book = $excel.Workbooks.Open($resolvedInputPath, 3, $true)
            try {
                $book.ExportAsFixedFormat(0, $resolvedOutputPath)
            } finally {
                $book.Close($false)
            }
        } finally {
            $excel.Quit()
        }
    }
    ".ppt" {
        $powerPoint = New-Object -ComObject PowerPoint.Application
        try {
            $presentation = $powerPoint.Presentations.Open($resolvedInputPath, $true, $false, $false)
            try {
                $presentation.SaveAs($resolvedOutputPath, 32)
            } finally {
                $presentation.Close()
            }
        } finally {
            $powerPoint.Quit()
        }
    }
    ".pptx" {
        $powerPoint = New-Object -ComObject PowerPoint.Application
        try {
            $presentation = $powerPoint.Presentations.Open($resolvedInputPath, $true, $false, $false)
            try {
                $presentation.SaveAs($resolvedOutputPath, 32)
            } finally {
                $presentation.Close()
            }
        } finally {
            $powerPoint.Quit()
        }
    }
    { $_ -in ".jpg", ".jpeg", ".png", ".bmp" } {
        # The target printer is monochrome. Build a grayscale bitmap first so
        # the preview PDF is exactly the tonal version that will be submitted.
        Add-Type -AssemblyName System.Drawing
        $sourceImage = [System.Drawing.Image]::FromFile($resolvedInputPath)
        $imageWidth = $sourceImage.Width
        $imageHeight = $sourceImage.Height
        $grayPath = [System.IO.Path]::Combine(
            [System.IO.Path]::GetTempPath(),
            "print-server-$([System.Guid]::NewGuid()).png"
        )
        try {
            $grayBitmap = New-Object System.Drawing.Bitmap($imageWidth, $imageHeight)
            try {
                $graphics = [System.Drawing.Graphics]::FromImage($grayBitmap)
                try {
                    $matrix = New-Object System.Drawing.Imaging.ColorMatrix
                    $matrix.Matrix00 = 0.299
                    $matrix.Matrix01 = 0.299
                    $matrix.Matrix02 = 0.299
                    $matrix.Matrix10 = 0.587
                    $matrix.Matrix11 = 0.587
                    $matrix.Matrix12 = 0.587
                    $matrix.Matrix20 = 0.114
                    $matrix.Matrix21 = 0.114
                    $matrix.Matrix22 = 0.114
                    $attributes = New-Object System.Drawing.Imaging.ImageAttributes
                    try {
                        $attributes.SetColorMatrix($matrix)
                        $destination = New-Object System.Drawing.Rectangle(0, 0, $imageWidth, $imageHeight)
                        $graphics.DrawImage(
                            $sourceImage,
                            $destination,
                            0,
                            0,
                            $imageWidth,
                            $imageHeight,
                            [System.Drawing.GraphicsUnit]::Pixel,
                            $attributes
                        )
                    } finally {
                        $attributes.Dispose()
                    }
                } finally {
                    $graphics.Dispose()
                }
                $grayBitmap.Save($grayPath, [System.Drawing.Imaging.ImageFormat]::Png)
            } finally {
                $grayBitmap.Dispose()
            }
        } finally {
            $sourceImage.Dispose()
        }

        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        try {
            $doc = $word.Documents.Add()
            try {
                $section = $doc.Sections.Item(1)
                $pageSetup = $section.PageSetup
                $pageSetup.TopMargin = 36
                $pageSetup.BottomMargin = 36
                $pageSetup.LeftMargin = 36
                $pageSetup.RightMargin = 36
                if ($imageWidth -gt $imageHeight) {
                    $pageSetup.Orientation = 1
                }
                $shape = $doc.InlineShapes.AddPicture($grayPath)
                $shape.LockAspectRatio = -1
                $maxWidth = $pageSetup.PageWidth - $pageSetup.LeftMargin - $pageSetup.RightMargin
                $maxHeight = $pageSetup.PageHeight - $pageSetup.TopMargin - $pageSetup.BottomMargin
                $scale = [Math]::Min($maxWidth / $shape.Width, $maxHeight / $shape.Height)
                if ($scale -lt 1) {
                    $shape.Width = $shape.Width * $scale
                    $shape.Height = $shape.Height * $scale
                }
                $doc.Paragraphs.Item(1).Alignment = 1
                $doc.ExportAsFixedFormat($resolvedOutputPath, 17)
            } finally {
                $doc.Close($false)
            }
        } finally {
            $word.Quit()
            Remove-Item -LiteralPath $grayPath -Force -ErrorAction SilentlyContinue
        }
    }
    ".txt" {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        try {
            $doc = $word.Documents.Add()
            try {
                $doc.Content.Text = [System.IO.File]::ReadAllText($resolvedInputPath)
                $doc.ExportAsFixedFormat($resolvedOutputPath, 17)
            } finally {
                $doc.Close($false)
            }
        } finally {
            $word.Quit()
        }
    }
    default {
        throw "Unsupported Office file extension: $extension"
    }
}

if (-not [System.IO.File]::Exists($resolvedOutputPath)) {
    throw "Converter did not create output: $resolvedOutputPath"
}
